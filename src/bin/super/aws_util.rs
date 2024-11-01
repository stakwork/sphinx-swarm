use anyhow::Error;
use aws_config::meta::region::RegionProviderChain;
use aws_config::timeout::TimeoutConfig;
use aws_config::Region;
use aws_sdk_ec2::Client;
use aws_smithy_types::retry::RetryConfig;
use sphinx_swarm::utils::getenv;
use tokio::time::Duration;

pub async fn make_aws_client() -> Result<Client, Error> {
    let region = getenv("AWS_S3_REGION_NAME")?;
    let region_provider = RegionProviderChain::first_try(Some(Region::new(region)));
    let timeout_config = TimeoutConfig::builder()
        .connect_timeout(Duration::from_secs(5))
        .read_timeout(Duration::from_secs(60))
        .build();
    let config = aws_config::from_env()
        .region(region_provider)
        .retry_config(RetryConfig::standard().with_max_attempts(10))
        .timeout_config(timeout_config)
        .load()
        .await;

    Ok(Client::new(&config))
}
