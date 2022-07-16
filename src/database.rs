use rusqlite::{params, OptionalExtension, Params};
use serde::Serialize;
use tokio::sync::{
    mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    oneshot,
};

#[derive(Debug)]
enum Task {
    AddQuote(oneshot::Sender<bool>, Quote),
    GetQuote(oneshot::Sender<Option<Quote>>, Option<String>),
    SearchQuotes(oneshot::Sender<Option<Vec<Quote>>>, String),
    RandomNQuotes(oneshot::Sender<Option<Vec<Quote>>>, u8),
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
                    if let Err(e) = self.db.execute(
                        "insert into quotes(quote,username) values(?,?)",
                        params![quote.quote, quote.author],
                    ) {
                        tracing::error!("A database error has occurred: {}", e);
                        tx.send(false).unwrap();
                    } else {
                        tx.send(true).unwrap();
                    }
                }
                Task::GetQuote(tx, author) => {
                    let quote = if let Some(ref author) = author {
                        self.db.query_row("select quote,username from quotes where username=? order by random() limit 1", params![author], |v| Ok(Quote {quote:v.get(0)?, author:v.get(1)?}))
                    } else {
                        self.db.query_row("select quote,username from quotes order by random() limit 1", params![], |v| Ok(Quote {quote:v.get(0)?, author:v.get(1)?}))
                    }.optional().unwrap_or_else(|e| {
                        tracing::error!("A database error has occurred: {}", e);
                        None
                    });
                    tx.send(quote).unwrap();
                }
                Task::SearchQuotes(tx, query) => {
                    tx.send(self.yield_quotes("select quote,username from quotes where quote like '%'||?1||'%' order by quote asc limit 5", params![query])).unwrap();
                }
                Task::RandomNQuotes(tx, count) => {
                    tx.send(self.yield_quotes(
                        "select quote,username from quotes order by random() limit ?",
                        params![count],
                    ))
                    .unwrap();
                }
            }
        }
    }

    fn yield_quotes<P: Params>(&self, sql: &str, params: P) -> Option<Vec<Quote>> {
        match self.db.prepare(sql).and_then(|mut v| {
            v.query(params).and_then(|mut v| {
                let mut quotes: Vec<Quote> = Vec::with_capacity(50);
                while let Some(row) = v.next()? {
                    quotes.push(Quote {
                        quote: row.get(0)?,
                        author: row.get(1)?,
                    });
                }
                Ok(quotes)
            })
        }) {
            Ok(o) => Some(o),
            Err(e) => {
                tracing::error!("A database error has occurred: {}", e);
                None
            }
        }
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
    executor_wrapper!(add_quote, Task::AddQuote, bool, quote: Quote);
    executor_wrapper!(
        get_quote,
        Task::GetQuote,
        Option<Quote>,
        author: Option<String>
    );
    executor_wrapper!(
        search_quotes,
        Task::SearchQuotes,
        Option<Vec<Quote>>,
        query: String
    );
    executor_wrapper!(
        random_n_quotes,
        Task::RandomNQuotes,
        Option<Vec<Quote>>,
        count: u8
    );
}
