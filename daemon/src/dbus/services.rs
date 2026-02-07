use crate::nftables::engine::NftEngine;
use crate::nftables::model::RulesetSnapshot;
use crate::nftables::ownership::detect_table_owner;
use crate::services::detector::ServiceDetector;
use std::sync::Arc;

#[derive(Clone)]
pub struct ServicesService {
    detector: Arc<ServiceDetector>,
    engine: Arc<NftEngine>,
}

impl ServicesService {
    pub fn new(detector: Arc<ServiceDetector>, engine: Arc<NftEngine>) -> Self {
        Self { detector, engine }
    }
}

#[zbus::interface(name = "org.palisade.Daemon1.Services")]
impl ServicesService {
    async fn detect_services(&self) -> zbus::fdo::Result<String> {
        serde_json::to_string(&self.detector.detect())
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    async fn get_table_ownership(&self) -> zbus::fdo::Result<String> {
        let raw = self
            .engine
            .list_ruleset()
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        let snapshot = RulesetSnapshot::from_nft_json(&raw)
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        let value = snapshot
            .tables
            .iter()
            .map(|t| {
                (
                    format!("{}/{}", t.family, t.name),
                    format!("{:?}", detect_table_owner(&t.family, &t.name)),
                )
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        serde_json::to_string(&value).map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }
}
