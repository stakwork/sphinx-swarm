use anyhow::{anyhow, Error};
use aws_config::meta::region::RegionProviderChain;
use aws_config::Region;
use aws_sdk_route53::types::{
    Change, ChangeAction, ChangeBatch, ResourceRecord, ResourceRecordSet,
};
use aws_sdk_route53::Client as Route53Client;
use aws_smithy_types::retry::RetryConfig;
use sphinx_swarm::utils::getenv;

pub async fn add_domain_name_to_route53(
    domain_names: Vec<&str>,
    public_ip: &str,
) -> Result<(), Error> {
    let region = getenv("AWS_S3_REGION_NAME")?;
    let hosted_zone_id = getenv("ROUTE53_ZONE_ID")?;
    let region_provider = RegionProviderChain::first_try(Some(Region::new(region)));
    let config = aws_config::from_env()
        .region(region_provider)
        .retry_config(RetryConfig::standard().with_max_attempts(10))
        .load()
        .await;
    let route53_client = Route53Client::new(&config);

    let mut changes = Vec::new();

    for &domain in &domain_names {
        let resource_record = ResourceRecord::builder().value(public_ip).build()?;

        let resource_record_set = ResourceRecordSet::builder()
            .name(domain)
            .r#type("A".into()) // A record for IPv4
            .ttl(300) // Time-to-live (in seconds)
            .resource_records(resource_record)
            .build()
            .map_err(|err| anyhow!(err.to_string()))?;

        // Create a change request to upsert (create or update) the A record
        let change = Change::builder()
            .action(ChangeAction::Upsert)
            .resource_record_set(resource_record_set)
            .build()
            .map_err(|err| anyhow!(err.to_string()))?;

        changes.push(change);
    }

    let change_batch = ChangeBatch::builder()
        .set_changes(Some(changes))
        .build()
        .map_err(|err| anyhow!(err.to_string()))?;

    let response = route53_client
        .change_resource_record_sets()
        .hosted_zone_id(hosted_zone_id)
        .change_batch(change_batch)
        .send()
        .await?;

    log::info!(
        "Route 53 change status for {:?}: {:?}",
        domain_names,
        response.change_info()
    );

    Ok(())
}
