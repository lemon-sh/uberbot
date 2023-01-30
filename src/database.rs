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

#[derive(Debug)]
enum Task {
    AddQuote(oneshot::Sender<rusqlite::Result<()>>, Quote),
    GetQuote(
        oneshot::Sender<rusqlite::Result<Option<Quote>>>,
        Option<String>,
    ),
    StartSearch(
        oneshot::Sender<rusqlite::Result<Vec<Quote>>>,
        String,
        String,
        usize,
    ),
    NextSearch(
        oneshot::Sender<rusqlite::Result<Option<Vec<Quote>>>>,
        String,
        usize,
    ),
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
        Ok((Self { rx, db }, ExecutorConnection { tx }))
    }

    pub fn run(mut self) {
        let mut searches: HashMap<String, (String, i64)> = HashMap::new();
        while let Some(task) = self.rx.blocking_recv() {
            let before = Instant::now();
            tracing::debug!("got task {:?}", task);
            match task {
                Task::AddQuote(tx, mut quote) => {
                    quote.author.make_ascii_lowercase();
                    let result = self
                        .db
                        .execute(
                            "insert into quotes(quote,username) values(?,?)",
                            params![quote.quote, quote.author],
                        )
                        .map(|_| ());
                    tx.send(result).unwrap();
                }
                Task::GetQuote(tx, author) => {
                    let result = if let Some(mut author) = author {
                        author.make_ascii_lowercase();
                        self.db.query_row("select quote,username from quotes where username match ? order by random() limit 1", params![author], |v| Ok(Quote {quote:v.get(0)?, author:v.get(1)?}))
                    } else {
                        self.db.query_row("select quote,username from quotes order by random() limit 1", params![], |v| Ok(Quote {quote:v.get(0)?, author:v.get(1)?}))
                    }.optional();
                    tx.send(result).unwrap();
                }
                Task::StartSearch(tx, user, query, limit) => {
                    tx.send(self.start_search(&mut searches, user, query, limit))
                        .unwrap();
                }
                Task::NextSearch(tx, user, limit) => {
                    tx.send(self.next_search(&mut searches, &user, limit))
                        .unwrap();
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

pub struct ExecutorConnection {
    tx: UnboundedSender<Task>,
}

impl Clone for ExecutorConnection {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}

// TODO: this is ugly, write a macro that will generate
//       both the Task enum and the ExecutorConnection impl.
macro_rules! executor_wrapper {
    ($name:ident, $task:expr, $ret:ty, $($arg:ident: $ty:ty),*) => {
        pub async fn $name(&self, $($arg: $ty),*) -> $ret {
          let (otx, orx) = oneshot::channel();
          self.tx.send($task(otx, $($arg),*)).unwrap();
          orx.await.unwrap()
        }
    };
    ($name:ident, $task:expr, $ret:ty) => {
        pub async fn $name(&self) -> $ret {
          let (otx, orx) = oneshot::channel();
          self.tx.send($task(otx)).unwrap();
          orx.await.unwrap()
        }
    };
}

impl ExecutorConnection {
    // WARNING: these methods are NOT cancel-safe
    executor_wrapper!(
        add_quote,
        Task::AddQuote,
        rusqlite::Result<()>,
        quote: Quote
    );
    executor_wrapper!(
        get_quote,
        Task::GetQuote,
        rusqlite::Result<Option<Quote>>,
        author: Option<String>
    );
    executor_wrapper!(
        search_quotes,
        Task::StartSearch,
        rusqlite::Result<Vec<Quote>>,
        user: String,
        query: String,
        limit: usize
    );
    executor_wrapper!(
        advance_search,
        Task::NextSearch,
        rusqlite::Result<Option<Vec<Quote>>>,
        user: String,
        limit: usize
    );
}
