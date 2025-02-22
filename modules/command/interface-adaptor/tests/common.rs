use std::future::Future;
use std::time::Duration;
use std::{env, thread};

use anyhow::Result;
use aws_sdk_dynamodb::config::{Credentials, Region};
use aws_sdk_dynamodb::operation::create_table::CreateTableOutput;
use aws_sdk_dynamodb::types::{
  AttributeDefinition, GlobalSecondaryIndex, KeySchemaElement, KeyType, Projection, ProjectionType,
  ProvisionedThroughput, ScalarAttributeType,
};
use aws_sdk_dynamodb::Client;
use testcontainers::clients;
use testcontainers::core::WaitFor;
use testcontainers::images::generic::GenericImage;

use cqrs_es_example_command_interface_adaptor::gateways::event_persistence_gateway::EventPersistenceGateway;
use cqrs_es_example_command_interface_adaptor::gateways::thread_repository::ThreadRepositoryImpl;

pub fn init_logger() {
  let _ = env::set_var("RUST_LOG", "debug");
  let _ = env_logger::builder().is_test(true).try_init();
}

pub async fn create_journal_table(client: &Client, table_name: &str, gsi_name: &str) -> Result<CreateTableOutput> {
  let pkey_attribute_definition = AttributeDefinition::builder()
    .attribute_name("pkey")
    .attribute_type(ScalarAttributeType::S)
    .build();

  let skey_attribute_definition = AttributeDefinition::builder()
    .attribute_name("skey")
    .attribute_type(ScalarAttributeType::S)
    .build();

  let pkey_schema = KeySchemaElement::builder()
    .attribute_name("pkey")
    .key_type(KeyType::Hash)
    .build();

  let skey_schema = KeySchemaElement::builder()
    .attribute_name("skey")
    .key_type(KeyType::Range)
    .build();

  let aid_attribute_definition = AttributeDefinition::builder()
    .attribute_name("aid")
    .attribute_type(ScalarAttributeType::S)
    .build();

  let seq_nr_attribute_definition = AttributeDefinition::builder()
    .attribute_name("seq_nr")
    .attribute_type(ScalarAttributeType::N)
    .build();

  let provisioned_throughput = ProvisionedThroughput::builder()
    .read_capacity_units(10)
    .write_capacity_units(5)
    .build();

  let gsi = GlobalSecondaryIndex::builder()
    .index_name(gsi_name)
    .key_schema(
      KeySchemaElement::builder()
        .attribute_name("aid")
        .key_type(KeyType::Hash)
        .build(),
    )
    .key_schema(
      KeySchemaElement::builder()
        .attribute_name("seq_nr")
        .key_type(KeyType::Range)
        .build(),
    )
    .projection(Projection::builder().projection_type(ProjectionType::All).build())
    .provisioned_throughput(provisioned_throughput.clone())
    .build();

  let result = client
    .create_table()
    .table_name(table_name)
    .attribute_definitions(pkey_attribute_definition)
    .attribute_definitions(skey_attribute_definition)
    .attribute_definitions(aid_attribute_definition)
    .attribute_definitions(seq_nr_attribute_definition)
    .key_schema(pkey_schema)
    .key_schema(skey_schema)
    .global_secondary_indexes(gsi)
    .provisioned_throughput(provisioned_throughput)
    .send()
    .await?;

  Ok(result)
}

pub async fn create_snapshot_table(client: &Client, table_name: &str, gsi_name: &str) -> Result<CreateTableOutput> {
  let pkey_attribute_definition = AttributeDefinition::builder()
    .attribute_name("pkey")
    .attribute_type(ScalarAttributeType::S)
    .build();

  let pkey_schema = KeySchemaElement::builder()
    .attribute_name("pkey")
    .key_type(KeyType::Hash)
    .build();

  let skey_attribute_definition = AttributeDefinition::builder()
    .attribute_name("skey")
    .attribute_type(ScalarAttributeType::S)
    .build();

  let skey_schema = KeySchemaElement::builder()
    .attribute_name("skey")
    .key_type(KeyType::Range)
    .build();

  let aid_attribute_definition = AttributeDefinition::builder()
    .attribute_name("aid")
    .attribute_type(ScalarAttributeType::S)
    .build();

  let seq_nr_attribute_definition = AttributeDefinition::builder()
    .attribute_name("seq_nr")
    .attribute_type(ScalarAttributeType::N)
    .build();

  let provisioned_throughput = ProvisionedThroughput::builder()
    .read_capacity_units(10)
    .write_capacity_units(5)
    .build();

  let gsi = GlobalSecondaryIndex::builder()
    .index_name(gsi_name)
    .key_schema(
      KeySchemaElement::builder()
        .attribute_name("aid")
        .key_type(KeyType::Hash)
        .build(),
    )
    .key_schema(
      KeySchemaElement::builder()
        .attribute_name("seq_nr")
        .key_type(KeyType::Range)
        .build(),
    )
    .projection(Projection::builder().projection_type(ProjectionType::All).build())
    .provisioned_throughput(provisioned_throughput.clone())
    .build();

  let result = client
    .create_table()
    .table_name(table_name)
    .attribute_definitions(pkey_attribute_definition)
    .attribute_definitions(skey_attribute_definition)
    .attribute_definitions(aid_attribute_definition)
    .attribute_definitions(seq_nr_attribute_definition)
    .key_schema(pkey_schema)
    .key_schema(skey_schema)
    .global_secondary_indexes(gsi)
    .provisioned_throughput(provisioned_throughput)
    .send()
    .await?;

  Ok(result)
}
pub fn create_client(dynamodb_port: u16) -> Client {
  let region = Region::new("us-west-1");
  let config = aws_sdk_dynamodb::Config::builder()
    .region(Some(region))
    .endpoint_url(format!("http://localhost:{}", dynamodb_port))
    .credentials_provider(Credentials::new("x", "x", None, None, "default"))
    .build();
  let client = Client::from_conf(config);
  client
}

async fn wait_table(client: &Client, target_table_name: &str) -> bool {
  let lto = client.list_tables().send().await;
  match lto {
    Ok(lto) => match lto.table_names() {
      Some(table_names) => table_names.iter().any(|tn| tn == target_table_name),
      None => false,
    },
    Err(e) => {
      println!("Error: {}", e);
      false
    }
  }
}

pub async fn with_repository<F, Fut>(f: F)
where
  F: Fn(ThreadRepositoryImpl) -> Fut,
  Fut: Future<Output = ()>,
{
  init_logger();
  let docker = clients::Cli::default();
  let wait_for = WaitFor::message_on_stdout("Port:");
  let image = GenericImage::new("amazon/dynamodb-local", "1.18.0").with_wait_for(wait_for);
  let dynamodb_node = docker.run::<GenericImage>(image);
  let port = dynamodb_node.get_host_port_ipv4(8000);
  log::debug!("DynamoDB port: {}", port);
  let client = create_client(port);

  let journal_table_name = "journal";
  let journal_aid_index_name = "journal-aid-index";
  let _ = create_journal_table(&client, journal_table_name, journal_aid_index_name).await;

  let snapshot_table_name = "snapshot";
  let snapshot_aid_index_name = "snapshot-aid-index";
  let _ = create_snapshot_table(&client, snapshot_table_name, snapshot_aid_index_name).await;

  while wait_table(&client, journal_table_name).await == false {
    thread::sleep(Duration::from_millis(1000));
  }

  while wait_table(&client, snapshot_table_name).await == false {
    thread::sleep(Duration::from_millis(1000));
  }

  let epg = EventPersistenceGateway::new(
    client,
    journal_table_name.to_string(),
    journal_aid_index_name.to_string(),
    snapshot_table_name.to_string(),
    snapshot_aid_index_name.to_string(),
    64,
  );
  let repository = ThreadRepositoryImpl::new(epg);
  f(repository).await;
}
