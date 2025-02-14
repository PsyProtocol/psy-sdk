use anyhow::Result;
use scylla::{Session, SessionBuilder};
use std::env;
use std::sync::Arc;

use tokio::sync::Semaphore;

use std::time::Duration;
use temporal_sdk::{ActivityOptions, WfContext, WfExitValue, WorkflowResult};
use temporal_sdk_core::protos::temporal::api::common::v1::RetryPolicy;
use temporal_sdk_core_protos::coresdk::AsJsonPayloadExt;
use prost_wkt_types::Duration as ProstDuration;
use log::{debug, info, warn};



use temporal_sdk_core::protos::coresdk::activity_result::{
    activity_resolution::Status::Completed, ActivityResolution,
};

pub fn parse_activity_result<'a, T>(result: &'a ActivityResolution) -> Result<T, anyhow::Error>
where
    T: serde::Deserialize<'a>,
{
    if result.completed_ok() {
        if let Some(Completed(result)) = &result.status {
            if let Some(payload) = &result.result {
                // let data = from_utf8(&payload.data).unwrap();
                let result: T = serde_json::from_slice(&payload.data).unwrap();
                // println!("Activity completed with: {:#?}", string_result.to_owned());
                return Ok(result);
            }
        } else {
            debug!("Activity failed with {:?}", result.status);
        }
    }
    Err(anyhow::anyhow!("Activity failed"))
}

pub async fn http_workflow(ctx: WfContext) -> WorkflowResult<String> {
    debug!("Inside http workflow");
    let act_handle = ctx
        .activity(ActivityOptions {
            activity_type: "make_http_request".to_string(),
            input: "".as_json_payload()?, // no actual payload
            retry_policy: Some(RetryPolicy {
                initial_interval: Some(ProstDuration {
                    seconds: 0,
                    nanos: 50_000_000, // 50ms
                }),
                maximum_attempts: 2,
                ..Default::default()
            }),
            start_to_close_timeout: Some(Duration::from_secs(30)),
            ..Default::default()
        })
        .await;

    match parse_activity_result::<String>(&act_handle) {
        Ok(result) => {
            info!("Activity completed with: {:#?}", result);
            Ok(WfExitValue::Normal(result))
        }
        Err(_) => {
            warn!("Activity failed");
            Ok(WfExitValue::Evicted)
        }
    }
}

pub async fn async_activity_workflow(ctx: WfContext) -> WorkflowResult<String> {
    debug!("Inside async activity workflow");
    let act_handle = ctx
        .activity(ActivityOptions {
            activity_type: "do_something_async".to_string(),
            input: "".as_json_payload()?, // no actual payload
            retry_policy: Some(RetryPolicy {
                initial_interval: Some(ProstDuration {
                    seconds: 0,
                    nanos: 50_000_000, // 50ms
                }),
                maximum_attempts: 2,
                ..Default::default()
            }),
            start_to_close_timeout: Some(Duration::from_secs(30)),
            ..Default::default()
        })
        .await;

    match parse_activity_result::<String>(&act_handle) {
        Ok(result) => {
            info!("Activity completed with: {:#?}", result);
            Ok(WfExitValue::Normal(result))
        }
        Err(_) => {
            warn!("Activity failed");
            Ok(WfExitValue::Evicted)
        }
    }
}



#[tokio::main]
async fn main() -> Result<()> {
    println!("Ok.");

    Ok(())

}