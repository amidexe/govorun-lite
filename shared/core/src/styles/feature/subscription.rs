//! Storage continuation of the existing Core starter activation. No payments.
use super::{activation::core, Effect, StylesFeature};
use serde_json::{json, Value};

impl StylesFeature {
    pub async fn activation_options(
        &self,
        generation: u64,
        supplied: Value,
    ) -> Result<Value, &'static str> {
        let _serial = self.serial.lock().await;
        let facts = (self.facts)().await?.access;
        if facts["generation"] != generation || self.state.lock().unwrap().stopped {
            return Err("styles_session_changed");
        }
        let library = self
            .library
            .execute(json!({"kind":"snapshot"}), None, false)
            .await?;
        let result = core(
            json!({"operation":"styleLibrary","command":{"kind":"activationOptions",
            "context":library["configuration"]["context"],"facts":{
                "localTermsVersion":library["configuration"]["metadata"]["localTermsVersion"],
                "optionsPresent":library["storedOptionsPresent"]},"supplied":supplied}}),
        )?;
        if (self.facts)().await?.access["generation"] != generation {
            return Err("styles_session_changed");
        }
        Ok(result["activationOptions"].clone())
    }
    pub async fn apply_subscription(
        &self,
        generation: u64,
        effect: &Value,
    ) -> Result<(), &'static str> {
        let _serial = self.serial.lock().await;
        let facts = (self.facts)().await?.access;
        if facts["generation"] != generation || self.state.lock().unwrap().stopped {
            return Err("styles_session_changed");
        }
        if effect["kind"] != "activateStyle"
            || effect["termsVersion"] != facts["clientTermsVersion"]
        {
            return Err("styles_invalid_response");
        }
        let effects = self.effects.get().ok_or("styles_effects_unavailable")?;
        // The timestamp is confirmed Core authority; OS/UI cannot invent consent.
        let timestamp = (effects.io)(Effect::ConsentTimestamp { generation }).await?;
        if (self.facts)().await?.access["generation"] != generation {
            return Err("styles_session_changed");
        }
        self.library
            .execute(
                json!({"kind":"selectStyle","selection":effect["selection"],
            "activationOptions":effect["activationOptions"]}),
                None,
                true,
            )
            .await?;
        self.subscription_generation(generation).await?;
        self.library
            .execute(
                json!({"kind":"recordConsent","version":effect["termsVersion"],
            "acceptedAtMillis":timestamp}),
                None,
                true,
            )
            .await?;
        self.subscription_generation(generation).await?;
        self.library
            .execute(json!({"kind":"setEnabled","enabled":true}), None, true)
            .await?;
        if let Err(error) = self.subscription_generation(generation).await {
            self.library
                .execute(json!({"kind":"setEnabled","enabled":false}), None, true)
                .await?;
            return Err(error);
        }
        (effects.io)(Effect::PreparePasses { generation }).await?;
        let view = self.read_snapshot().await?;
        self.publish_view(view);
        Ok(())
    }
    async fn subscription_generation(&self, generation: u64) -> Result<(), &'static str> {
        if (self.facts)().await?.access["generation"] == generation
            && !self.state.lock().unwrap().stopped
        {
            Ok(())
        } else {
            Err("styles_session_changed")
        }
    }
}
