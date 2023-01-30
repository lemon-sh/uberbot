use rusqlite::{params, OptionalExtension, Params};
use serde::Serialize;
use std::collections::HashMap;
use tokio::{
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        oneshot,
    },
    time::Instant,
};

pub struct ExecutorConnection(UnboundedSender<Task>);
impl Clone for ExecutorConnection {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

macro_rules! executor_wrapper {
    ($($task:ident / $fn:ident, ($($arg:ident: $ty:ty),*) => $ret:ty)*) => {
        #[derive(Debug)]
        enum Task {
            $($task{tx:oneshot::Sender<$ret>,$($arg:$ty,)*}),*
        }

        impl ExecutorConnection {
            $(pub async fn $fn(&self, $($arg: $ty),*) -> $ret {
                let (tx, rx) = oneshot::channel();
                self.0.send(Task::$task{tx,$($arg),*}).unwrap();
                rx.await.unwrap()
            })*
        }
    };
}

executor_wrapper! {
    AddQuote / add_quote, (quote: Quote) => rusqlite::Result<()>
    GetQuote / get_quote, (author: Option<String>) => rusqlite::Result<Option<Quote>>
    StartSearch / search_quotes, (user: String, query: String, limit: usize) => rusqlite::Result<Vec<Quote>>
    NextSearch / advance_search, (user: String, limit: usize) => rusqlite::Result<Option<Vec<Quote>>>
}

pub struct DbExecutor {
    rx: UnboundedReceiver<Task>,
    db: rusqlite::Connection,
}

#[derive(Serialize, Debug)]
pub struct Quote {
    pub author: String,
    pub quote: String,
}

impl DbExecutor {
    pub fn create(dbpath: &str) -> rusqlite::Result<(Self, ExecutorConnection)> {
        let (tx, rx) = unbounded_channel();
        let db = rusqlite::Connection::open(dbpath)?;
        db.execute(
            "create virtual table if not exists quotes using fts5(username, quote)",
            [],
        )?;
        tracing::debug!("Database connected ({})", dbpath);
        Ok((Self { rx, db }, ExecutorConnection(tx)))
    }

    pub fn run(mut self) {
        let mut searches: HashMap<String, (String, i64)> = HashMap::new();
        while let Some(task) = self.rx.blocking_recv() {
            let before = Instant::now();
            tracing::debug!("got task {:?}", task);
            match task {
                Task::AddQuote { tx, mut quote } => {
                    quote.author.make_ascii_lowercase();
                    let result = self
                        .db
                        .execute(
                            "insert into quotes(quote,username) values(?,?)",
                            params![quote.quote, quote.author],
                        )
                        .map(|_| ());
                    let _e = tx.send(result);
                }
                Task::GetQuote { tx, author } => {
                    let result = if let Some(mut author) = author {
                        author.make_ascii_lowercase();
                        self.db.query_row("select quote,username from quotes where username match ? order by random() limit 1", params![author], |v| Ok(Quote {quote:v.get(0)?, author:v.get(1)?}))
                    } else {
                        self.db.query_row("select quote,username from quotes order by random() limit 1", params![], |v| Ok(Quote {quote:v.get(0)?, author:v.get(1)?}))
                    }.optional();
                    let _e = tx.send(result);
                }
                Task::StartSearch {
                    tx,
                    user,
                    query,
                    limit,
                } => {
                    let _e = tx.send(self.start_search(&mut searches, user, query, limit));
                }
                Task::NextSearch { tx, user, limit } => {
                    let _e = tx.send(self.next_search(&mut searches, &user, limit));
                }
            }
            tracing::debug!(
                "task took {}ms",
                Instant::now().duration_since(before).as_secs_f64() / 1000.0
            );
        }
    }

    fn start_search(
        &self,
        searches: &mut HashMap<String, (String, i64)>,
        user: String,
        query: String,
        limit: usize,
    ) -> rusqlite::Result<Vec<Quote>> {
        let (quotes, oid) = self.yield_quotes_oid(
            "select oid,quote,username from quotes where quote match ? order by oid asc limit ?",
            params![query, limit],
        )?;
        searches.insert(user, (query, oid));
        Ok(quotes)
    }

    fn next_search(
        &self,
        searches: &mut HashMap<String, (String, i64)>,
        user: &str,
        limit: usize,
    ) -> rusqlite::Result<Option<Vec<Quote>>> {
        let Some((query, old_oid)) = searches.get_mut(user) else { return Ok(None); };
        let (quotes, new_oid) = self.yield_quotes_oid("select oid,quote,username from quotes where oid > ? and quote match ? order by oid asc limit ?", params![*old_oid, &*query, limit])?;
        if new_oid != -1 {
            *old_oid = new_oid;
        }
        Ok(Some(quotes))
    }

    fn yield_quotes_oid<P: Params>(
        &self,
        sql: &str,
        params: P,
    ) -> rusqlite::Result<(Vec<Quote>, i64)> {
        let mut lastoid = -1i64;
        let quotes = self.db.prepare(sql).and_then(|mut v| {
            v.query(params).and_then(|mut v| {
                let mut quotes: Vec<Quote> = Vec::new();
                while let Some(row) = v.next()? {
                    lastoid = row.get(0)?;
                    quotes.push(Quote {
                        quote: row.get(1)?,
                        author: row.get(2)?,
                    });
                }
                Ok(quotes)
            })
        })?;
        Ok((quotes, lastoid))
    }
}
