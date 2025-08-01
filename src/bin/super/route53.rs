use anyhow::{anyhow, Error};
use aws_config::meta::region::RegionProviderChain;
use aws_config::Region;
use aws_sdk_ec2::error::{ProvideErrorMetadata, SdkError};
use aws_sdk_route53::types::RrType;
use aws_sdk_route53::types::{
    Change, ChangeAction, ChangeBatch, ResourceRecord, ResourceRecordSet,
};
use aws_sdk_route53::Client as Route53Client;
use aws_smithy_types::retry::RetryConfig;
use sphinx_swarm::utils::getenv;

async fn make_route_53_client() -> Result<Route53Client, Error> {
    let region = getenv("AWS_REGION")?;
    let region_provider = RegionProviderChain::first_try(Some(Region::new(region)));
    let config = aws_config::from_env()
        .region(region_provider)
        .retry_config(RetryConfig::standard().with_max_attempts(10))
        .load()
        .await;
    Ok(Route53Client::new(&config))
}

pub async fn add_domain_name_to_route53(
    domain_names: Vec<String>,
    public_ip: &str,
) -> Result<(), Error> {
    let hosted_zone_id = getenv("ROUTE53_ZONE_ID")?;

    let route53_client = make_route_53_client().await?;

    let mut changes = Vec::new();

    for domain in &domain_names {
        let resource_record = ResourceRecord::builder().value(public_ip).build()?;

        let resource_record_set = ResourceRecordSet::builder()
            .name(domain.as_str())
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

    let result = route53_client
        .change_resource_record_sets()
        .hosted_zone_id(hosted_zone_id)
        .change_batch(change_batch)
        .send()
        .await;

    match result {
        Ok(response) => {
            log::info!(
                "Route 53 change status for {:?}: {:?}",
                domain_names,
                response.change_info()
            );

            return Ok(());
        }
        Err(SdkError::ServiceError(service_error)) => {
            let err = service_error
                .err()
                .message()
                .unwrap_or("Unknown error")
                .to_string();
            log::error!("Service error: {}", err);
            return Err(anyhow!(err));
        }
        Err(SdkError::TimeoutError(_)) => {
            let err_msg = "Request timed out.";
            log::error!("{}", err_msg);
            return Err(anyhow!(err_msg));
        }
        Err(SdkError::DispatchFailure(err)) => {
            log::error!("Network error: {:?}", err);
            return Err(anyhow!("Network error"));
        }
        Err(e) => {
            log::error!("Unexpected error: {:?}", e);
            return Err(anyhow!("Unexpected error"));
        }
    }
}

pub async fn domain_exists_in_route53(domain: &str) -> Result<bool, Error> {
    let hosted_zone_id = getenv("ROUTE53_ZONE_ID")?;

    let client = make_route_53_client().await?;

    let mut start_record_name = None;
    let mut start_record_type = None;

    loop {
        let mut request = client
            .list_resource_record_sets()
            .hosted_zone_id(&hosted_zone_id);

        if let Some(name) = start_record_name.clone() {
            request = request.start_record_name(name);
        }

        if let Some(rrtype) = start_record_type.clone() {
            request = request.start_record_type(rrtype);
        }

        let result = request.send().await?;

        for record_set in result.resource_record_sets() {
            let record_name = record_set.name().trim_end_matches('.');
            let record_type = record_set.r#type();

            if record_name == domain && *record_type == RrType::A {
                return Ok(true);
            }
        }

        if result.is_truncated() {
            start_record_name = result.next_record_name().map(|s| s.to_string());
            start_record_type = result.next_record_type().cloned();
        } else {
            break;
        }
    }

    Ok(false)
}
