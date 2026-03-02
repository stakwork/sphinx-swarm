use anyhow::{anyhow, Error, Result};
use aws_config::meta::region::RegionProviderChain;
use aws_config::Region;
use aws_sdk_cloudwatch::types::{ComparisonOperator, Dimension, Statistic};
use aws_sdk_cloudwatch::Client;
use sphinx_swarm::utils::getenv;

/// Create CloudWatch alarms for CPU and Memory monitoring
pub async fn create_cloudwatch_alarms(
    instance_id: &str,
    swarm_name: &str,
) -> Result<(String, String), Error> {
    let region = getenv("AWS_REGION")?;
    let region_provider = RegionProviderChain::first_try(Some(Region::new(region)));
    let config = aws_config::from_env()
        .region(region_provider)
        .load()
        .await;

    let client = Client::new(&config);

    // Create CPU alarm
    let cpu_alarm_name = format!("{}-cpu-high", swarm_name);
    let cpu_alarm_description = format!("CPU utilization is too high for swarm {}", swarm_name);

    log::info!("Creating CloudWatch CPU alarm: {}", cpu_alarm_name);

    client
        .put_metric_alarm()
        .alarm_name(&cpu_alarm_name)
        .alarm_description(&cpu_alarm_description)
        .comparison_operator(ComparisonOperator::GreaterThanThreshold)
        .evaluation_periods(2)
        .metric_name("CPUUtilization")
        .namespace("AWS/EC2")
        .period(300) // 5 minutes
        .statistic(Statistic::Average)
        .threshold(80.0) // 80% CPU threshold
        .dimensions(
            Dimension::builder()
                .name("InstanceId")
                .value(instance_id)
                .build(),
        )
        .send()
        .await
        .map_err(|e| anyhow!("Failed to create CPU alarm: {:?}", e))?;

    log::info!("CPU alarm created successfully: {}", cpu_alarm_name);

    // Create Memory alarm
    let memory_alarm_name = format!("{}-memory-high", swarm_name);
    let memory_alarm_description = format!("Memory utilization is too high for swarm {}", swarm_name);

    log::info!("Creating CloudWatch Memory alarm: {}", memory_alarm_name);

    client
        .put_metric_alarm()
        .alarm_name(&memory_alarm_name)
        .alarm_description(&memory_alarm_description)
        .comparison_operator(ComparisonOperator::GreaterThanThreshold)
        .evaluation_periods(2)
        .metric_name("mem_used_percent")
        .namespace("CWAgent")
        .period(300) // 5 minutes
        .statistic(Statistic::Average)
        .threshold(80.0) // 80% memory threshold
        .dimensions(
            Dimension::builder()
                .name("InstanceId")
                .value(instance_id)
                .build(),
        )
        .send()
        .await
        .map_err(|e| anyhow!("Failed to create memory alarm: {:?}", e))?;

    log::info!("Memory alarm created successfully: {}", memory_alarm_name);

    Ok((cpu_alarm_name, memory_alarm_name))
}

/// Delete CloudWatch alarms for a swarm
pub async fn delete_cloudwatch_alarms(swarm_name: &str) -> Result<(), Error> {
    let region = getenv("AWS_REGION")?;
    let region_provider = RegionProviderChain::first_try(Some(Region::new(region)));
    let config = aws_config::from_env()
        .region(region_provider)
        .load()
        .await;

    let client = Client::new(&config);

    let cpu_alarm_name = format!("{}-cpu-high", swarm_name);
    let memory_alarm_name = format!("{}-memory-high", swarm_name);

    log::info!("Deleting CloudWatch alarms for swarm: {}", swarm_name);

    client
        .delete_alarms()
        .alarm_names(&cpu_alarm_name)
        .alarm_names(&memory_alarm_name)
        .send()
        .await
        .map_err(|e| anyhow!("Failed to delete alarms: {:?}", e))?;

    log::info!("Alarms deleted successfully for swarm: {}", swarm_name);

    Ok(())
}
