use rusqlite::{params, OptionalExtension, Params};
use serde::Serialize;
use tokio::sync::{
    mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    oneshot,
};

#[derive(Debug)]
enum Task {
    AddQuote(oneshot::Sender<rusqlite::Result<()>>, Quote),
    GetQuote(
        oneshot::Sender<rusqlite::Result<Option<Quote>>>,
        Option<String>,
    ),
    SearchQuotes(oneshot::Sender<rusqlite::Result<Vec<Quote>>>, String),
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
            "create table if not exists quotes(id integer primary key, username text not null, quote text not null)",
            [],
        )?;
        tracing::debug!("Database connected ({})", dbpath);
        Ok((Self { rx, db }, ExecutorConnection { tx }))
    }

    pub fn run(mut self) {
        while let Some(task) = self.rx.blocking_recv() {
            match task {
                Task::AddQuote(tx, quote) => {
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
                    let result = if let Some(ref author) = author {
                        self.db.query_row("select quote,username from quotes where username=? order by random() limit 1", params![author], |v| Ok(Quote {quote:v.get(0)?, author:v.get(1)?}))
                    } else {
                        self.db.query_row("select quote,username from quotes order by random() limit 1", params![], |v| Ok(Quote {quote:v.get(0)?, author:v.get(1)?}))
                    }.optional();
                    tx.send(result).unwrap();
                }
                Task::SearchQuotes(tx, query) => {
                    tx.send(self.yield_quotes("select quote,username from quotes where quote like '%'||?1||'%' order by quote asc limit 5", params![query])).unwrap();
                }
            }
        }
    }

    fn yield_quotes<P: Params>(&self, sql: &str, params: P) -> rusqlite::Result<Vec<Quote>> {
        self.db.prepare(sql).and_then(|mut v| {
            v.query(params).and_then(|mut v| {
                let mut quotes: Vec<Quote> = Vec::new();
                while let Some(row) = v.next()? {
                    quotes.push(Quote {
                        quote: row.get(0)?,
                        author: row.get(1)?,
                    });
                }
                Ok(quotes)
            })
        })
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
        Task::SearchQuotes,
        rusqlite::Result<Vec<Quote>>,
        query: String
    );
}
