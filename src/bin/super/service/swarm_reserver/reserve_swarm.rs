use anyhow::Result;
use tokio::time::{sleep, Duration};

use crate::{
    ec2::get_instances_from_aws_by_swarm_tag_and_return_hash_map,
    route53::{add_domain_name_to_route53, delete_multiple_route53_records},
    service::swarm_reserver::utils::generate_random_secret,
    state::{self, default_reserved_instances, AvailableInstances},
    util::{create_ec2_instance, get_instance_ip},
};
pub async fn handle_reserve_swarms() -> Result<()> {
    let aws_instances_hashmap = get_instances_from_aws_by_swarm_tag_and_return_hash_map().await?;
    let mut state = state::STATE.lock().await;

    let mut domain_names_to_delete: Vec<String> = vec![];

    if state.reserved_instances.is_none() {
        state.reserved_instances = Some(default_reserved_instances())
    }

    state.reserved_instances.as_mut().unwrap().available_instances.retain(|reserved_instance| {
        if aws_instances_hashmap.contains_key(&reserved_instance.instance_id) {
            true
        } else {
            log::info!("Reserved instance with ID: {} no longer exists on AWS, removing from reserved instances", reserved_instance.instance_id);
            domain_names_to_delete.push(reserved_instance.host.clone());
            false
        }
    });
    let reserved_instances = state.reserved_instances.clone().unwrap();

    drop(state);
    match delete_multiple_route53_records(domain_names_to_delete).await {
        Ok(_) => {}
        Err(err) => {
            log::error!("Error deleting route53 records: {}", err.to_string());
        }
    };

    let amount_to_reserve =
        reserved_instances.minimum_available - reserved_instances.available_instances.len() as i32;

    if amount_to_reserve <= 0 {
        log::info!(
            "No need to reserve more instances at the moment, Amount tp reserver: {}",
            amount_to_reserve
        );
        return Ok(());
    }

    log::info!("Reserving {} instances", amount_to_reserve);

    for _ in 0..amount_to_reserve {
        // TODO: decide what we want to be the default instance type: ASK GONZALO
        let instance_type = "m6i.xlarge".to_string();
        let admin_password = generate_random_secret(16);

        let ec2_instance = create_ec2_instance(
            "".to_string(),
            None,
            instance_type.to_string().clone(),
            None,
            None,
            Some(admin_password.clone()),
        )
        .await?;

        sleep(Duration::from_secs(40)).await;

        let host = format!("swarm{}.sphinx.chat", ec2_instance.1);

        let domain_names: Vec<String> = vec![host.clone()];

        let ec2_ip_address = get_instance_ip(&ec2_instance.0).await?;

        let _ = add_domain_name_to_route53(domain_names.clone(), &ec2_ip_address).await?;

        let mut state = state::STATE.lock().await;

        state
            .reserved_instances
            .as_mut()
            .unwrap()
            .available_instances
            .push(AvailableInstances {
                instance_id: ec2_instance.0,
                instance_type: instance_type.to_string(),
                swarm_number: ec2_instance.1.to_string(),
                default_host: format!("swarm{}.sphinx.chat:8800", ec2_instance.1),
                user: None,
                pass: None,
                ip_address: Some(ec2_ip_address),
                host,
                admin_password,
            });

        drop(state)
    }

    Ok(())
}
