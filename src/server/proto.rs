use crate::core::task::TrackingTask;
use crate::core::types::State;
use crate::persistance::interface::Db;
use crate::stats::stats_server::Stats;
use crate::stats::{GetStatsRequest, GetStatsResponse};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
pub struct StatsService {
    db: Db,
}

impl StatsService {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

#[tonic::async_trait]
impl Stats for StatsService {
    type GetStatsStream = ReceiverStream<Result<GetStatsResponse, Status>>;

    async fn get_stats(
        &self,
        _req: Request<GetStatsRequest>,
    ) -> Result<Response<Self::GetStatsStream>, Status> {
        info!("get_stats entry point");

        let (tx, rx) = mpsc::channel(4);

        let mut db = self.db.clone();
        tokio::spawn(async move {
            let tasks = db
                .get_tasks_by_status(&[State::Created, State::Quit, State::Running, State::Stopped])
                .await;

            match tasks {
                Ok(tasks) => {
                    info!("get_stats: sending {} tasks", tasks.len());
                    for tt in tasks {
                        tx.send(Ok(get_stats_response_from_tts(tt))).await.unwrap();
                    }
                }
                Err(err) => error!("failed to get tasks in StatsServer: {:?}", err),
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

fn get_stats_response_from_tts(tt: TrackingTask) -> GetStatsResponse {
    GetStatsResponse {
        id: tt.id.to_string(),
        name: tt.name.unwrap_or_default(),
        spreadsheet_id: tt.spreadsheet_id,
        sheet: tt.sheet,
        direction: tt.direction.to_string(),
        interval_secs: 10 as i32,
        input: tt.input.unwrap_or_default().to_json(),
        status: tt.status.to_string(),
        eval_forest: tt.eval_forest.to_string().unwrap_or_default(),
        till_next_call: 0,
        currently_running: true,
    }
}
