use anyhow::{anyhow, Error};
use aws_config::meta::region::RegionProviderChain;
use aws_config::Region;
use aws_sdk_cloudwatch::types::ComparisonOperator;
use aws_sdk_cloudwatch::types::{Dimension, StandardUnit, Statistic};
use aws_sdk_cloudwatch::Client as CloudWatchClient;
use aws_sdk_cloudwatchlogs::Client as CloudWatchLogsClient;
use aws_sdk_sns::Client as SnsClient;
use sphinx_swarm::utils::getenv;

async fn make_cloudwatch_client() -> Result<CloudWatchClient, Error> {
    let region = getenv("AWS_REGION")?;
    let region_provider = RegionProviderChain::first_try(Some(Region::new(region)));
    let config = aws_config::from_env().region(region_provider).load().await;
    Ok(CloudWatchClient::new(&config))
}

async fn make_cloudwatch_logs_client() -> Result<CloudWatchLogsClient, Error> {
    let region = getenv("AWS_REGION")?;
    let region_provider = RegionProviderChain::first_try(Some(Region::new(region)));
    let config = aws_config::from_env().region(region_provider).load().await;
    Ok(CloudWatchLogsClient::new(&config))
}

pub async fn set_log_group_retention(log_group_name: &str) -> Result<(), Error> {
    let client = make_cloudwatch_logs_client().await?;
    client
        .put_retention_policy()
        .log_group_name(log_group_name)
        .retention_in_days(7)
        .send()
        .await
        .map_err(|e| anyhow!(e.to_string()))?;
    log::info!("Set 7-day retention on log group {}", log_group_name);
    Ok(())
}

async fn make_sns_client() -> Result<SnsClient, Error> {
    let region = getenv("AWS_REGION")?;
    let region_provider = RegionProviderChain::first_try(Some(Region::new(region)));
    let config = aws_config::from_env().region(region_provider).load().await;
    Ok(SnsClient::new(&config))
}

pub async fn ensure_sns_topic_and_subscription() -> Result<String, Error> {
    let sns = make_sns_client().await?;

    let topic_output = sns
        .create_topic()
        .name("sphinx-swarm-cpu-alerts")
        .send()
        .await
        .map_err(|e| anyhow!(e.to_string()))?;

    let arn = topic_output
        .topic_arn()
        .ok_or_else(|| anyhow!("SNS create_topic returned no ARN"))?
        .to_string();

    sns.subscribe()
        .topic_arn(&arn)
        .protocol("https")
        .endpoint("https://hive.sphinx.chat/admin/ec2/alerts")
        .send()
        .await
        .map_err(|e| anyhow!(e.to_string()))?;

    Ok(arn)
}

pub async fn create_cpu_alarms(
    instance_id: &str,
    swarm_number: &str,
    topic_arn: &str,
) -> Result<(), Error> {
    let cw = make_cloudwatch_client().await?;

    let dimension = Dimension::builder()
        .name("InstanceId")
        .value(instance_id)
        .build();

    // High CPU alarm: > 80% for 2 hours (24 x 5-min periods)
    cw.put_metric_alarm()
        .alarm_name(format!("swarm-{}-high-cpu", instance_id))
        .alarm_description(format!("High CPU for swarm {}", swarm_number))
        .namespace("AWS/EC2")
        .metric_name("CPUUtilization")
        .dimensions(dimension.clone())
        .comparison_operator(ComparisonOperator::GreaterThanThreshold)
        .threshold(80.0)
        .evaluation_periods(24)
        .period(300)
        .statistic(Statistic::Average)
        .alarm_actions(topic_arn)
        .send()
        .await
        .map_err(|e| anyhow!(e.to_string()))?;

    // Low CPU alarm: < 20% for 2 hours (24 x 5-min periods)
    cw.put_metric_alarm()
        .alarm_name(format!("swarm-{}-low-cpu", instance_id))
        .alarm_description(format!("Low CPU for swarm {}", swarm_number))
        .namespace("AWS/EC2")
        .metric_name("CPUUtilization")
        .dimensions(dimension)
        .comparison_operator(ComparisonOperator::LessThanThreshold)
        .threshold(20.0)
        .evaluation_periods(24)
        .period(300)
        .statistic(Statistic::Average)
        .alarm_actions(topic_arn)
        .send()
        .await
        .map_err(|e| anyhow!(e.to_string()))?;

    log::info!(
        "Created CloudWatch CPU alarms for instance {} (swarm {})",
        instance_id,
        swarm_number
    );

    Ok(())
}

pub async fn delete_cpu_alarms(instance_id: &str) -> Result<(), Error> {
    let cw = make_cloudwatch_client().await?;

    log::info!(
        "Deleting CloudWatch CPU alarms for instance {}",
        instance_id
    );

    cw.delete_alarms()
        .alarm_names(format!("swarm-{}-high-cpu", instance_id))
        .alarm_names(format!("swarm-{}-low-cpu", instance_id))
        .send()
        .await
        .map_err(|e| anyhow!(e.to_string()))?;

    Ok(())
}

pub async fn get_cpu_utilization(instance_id: &str) -> Result<Option<f64>, Error> {
    let cw = make_cloudwatch_client().await?;

    let now = aws_smithy_types::DateTime::from_secs(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
    );
    let start = aws_smithy_types::DateTime::from_secs(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
            - 600,
    );

    let dimension = Dimension::builder()
        .name("InstanceId")
        .value(instance_id)
        .build();

    let result = cw
        .get_metric_statistics()
        .namespace("AWS/EC2")
        .metric_name("CPUUtilization")
        .dimensions(dimension)
        .start_time(start)
        .end_time(now)
        .period(300)
        .statistics(Statistic::Average)
        .unit(StandardUnit::Percent)
        .send()
        .await
        .map_err(|e| anyhow!(e.to_string()))?;

    let mut datapoints = result.datapoints().to_vec();

    if datapoints.is_empty() {
        return Ok(None);
    }

    // Sort by timestamp descending — most recent first
    datapoints.sort_by(|a, b| {
        b.timestamp()
            .and_then(|bt| a.timestamp().map(|at| bt.secs().cmp(&at.secs())))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let most_recent = datapoints[0].average();
    Ok(most_recent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_sdk_cloudwatch::types::Datapoint;
    use aws_smithy_types::DateTime;

    fn make_datapoint(secs: i64, average: f64) -> Datapoint {
        Datapoint::builder()
            .timestamp(DateTime::from_secs(secs))
            .average(average)
            .build()
    }

    #[test]
    fn test_sort_datapoints_most_recent() {
        let mut datapoints = vec![
            make_datapoint(1000, 30.0),
            make_datapoint(3000, 75.0),
            make_datapoint(2000, 50.0),
        ];

        // Sort descending by timestamp
        datapoints.sort_by(|a, b| {
            b.timestamp()
                .and_then(|bt| a.timestamp().map(|at| bt.secs().cmp(&at.secs())))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Most recent (ts=3000) should be first
        assert_eq!(datapoints[0].average(), Some(75.0));
    }

    #[test]
    fn test_empty_datapoints_returns_none() {
        let datapoints: Vec<Datapoint> = vec![];
        let result: Option<f64> = if datapoints.is_empty() {
            None
        } else {
            datapoints[0].average()
        };
        assert_eq!(result, None);
    }
}
