use datatracker_rust::stats::stats_client::StatsClient;
use datatracker_rust::stats::GetStatsRequest;

#[tokio::main]
async fn main() {
    let mut client = StatsClient::connect("http://[::1]:10000").await.unwrap();

    let mut stream = client
        .get_stats(GetStatsRequest::default())
        .await
        .unwrap()
        .into_inner();

    while let Some(feature) = stream.message().await.unwrap() {
        println!("NOTE = {:?}", feature);
    }
}
