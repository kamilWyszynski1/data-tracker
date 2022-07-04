use crate::core::handler::Report;
use crate::schema::reports;
use diesel::{Insertable, Queryable};
use serde_json::json;

#[derive(Queryable, Insertable)]
#[table_name = "reports"]
pub struct ReportModel {
    pub task_id: String,
    pub phases: String,
    pub failed: bool,
}

impl ReportModel {
    pub fn from_report(report: &Report) -> Self {
        Self {
            task_id: report.task_id.to_string(),
            phases: json!(report.phases).to_string(),
            failed: !report.success,
        }
    }
}
