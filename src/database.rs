use rusqlite::{params, OptionalExtension};
use tokio::sync::{
    mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    oneshot,
};

#[derive(Debug)]
enum Task {
    AddQuote(oneshot::Sender<bool>, String, String),
    GetQuote(oneshot::Sender<Option<(String, String)>>, Option<String>),
    // implement search WITH PAGINATION
}

pub struct DbExecutor {
    rx: UnboundedReceiver<Task>,
    db: rusqlite::Connection,
}

impl DbExecutor {
    pub fn create(dbpath: &str) -> rusqlite::Result<(Self, ExecutorConnection)> {
        let (tx, rx) = unbounded_channel();
        let db = rusqlite::Connection::open(dbpath)?;
        db.execute(
            "create table if not exists quotes(id integer primary key,\
            username text not null, quote text not null)",
            [],
        )?;
        tracing::debug!("Database connected ({})", dbpath);
        Ok((Self { rx, db }, ExecutorConnection { tx }))
    }

    pub fn run(mut self) {
        while let Some(task) = self.rx.blocking_recv() {
            match task {
                Task::AddQuote(tx, quote, author) => {
                    if let Err(e) = self.db.execute(
                        "insert into quotes(quote,username) values(?,?)",
                        params![quote, author],
                    ) {
                        tracing::error!("A database error has occurred: {}", e);
                        tx.send(false).unwrap();
                    } else {
                        tx.send(true).unwrap();
                    }
                }
                Task::GetQuote(tx, author) => {
                    let quote = if let Some(ref author) = author {
                        self.db.query_row("select quote,username from quotes where username=? order by random() limit 1", params![author], |v| Ok((v.get(0)?, v.get(1)?)))
                    } else {
                        self.db.query_row("select quote,username from quotes order by random() limit 1", params![], |v| Ok((v.get(0)?, v.get(1)?)))
                    }.optional().unwrap_or_else(|e| {
                        tracing::error!("A database error has occurred: {}", e);
                        None
                    });
                    tx.send(quote).unwrap();
                }
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

impl ExecutorConnection {
    pub async fn add_quote(&self, quote: String, author: String) -> bool {
        let (otx, orx) = oneshot::channel();
        self.tx.send(Task::AddQuote(otx, quote, author)).unwrap();
        orx.await.unwrap()
    }
    pub async fn get_quote(&self, author: Option<String>) -> Option<(String, String)> {
        let (otx, orx) = oneshot::channel();
        self.tx.send(Task::GetQuote(otx, author)).unwrap();
        orx.await.unwrap()
    }
}
