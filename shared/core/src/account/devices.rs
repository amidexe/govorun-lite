use crate::{AccountRequest, AccountSession, Error, HttpResponse, Method, Result};
use serde_json::Value;

/// Server device inventory and publication ordering have one owner. Native
/// clients supply a display name, render this view, and open confirmations.
#[derive(Default)]
pub(crate) struct Devices {
    request: u64,
    pub list: Option<Value>,
}

impl AccountSession {
    pub(crate) async fn device_request(&self, epoch: u64, request: AccountRequest) -> Result<HttpResponse> {
        let (sequence, current_device) = {
            let mut state = self.state();
            if state.epoch != epoch { return Err(Error::SessionChanged); }
            let device = state.tokens.as_ref().ok_or(Error::SignedOut)?.device_id.clone();
            state.devices.request += 1;
            (state.devices.request, device)
        };
        let response = self.request_for_session(epoch, request.clone()).await;
        // A successful self-revocation intentionally ended this identity in the
        // request owner. Never require the ended session to remain signed in.
        if request.method() == Method::Delete &&
            request.path().strip_prefix("/v1/devices/") == Some(current_device.as_str()) {
            return response;
        }
        self.current(epoch)?;
        let response = match response {
            Ok(response) => response,
            Err(error) => { self.current_device_request(epoch, sequence)?; return Err(error); }
        };
        if !(200..300).contains(&response.status) {
            self.current_device_request(epoch, sequence)?;
            return Ok(response);
        }
        if request.method() == Method::Delete {
            // The server remains authoritative after another device is removed.
            // Retire the old list before I/O, so a failed reload cannot show the
            // revoked device as a confirmed current inventory.
            let sequence = {
                let mut state = self.mutate();
                if state.epoch != epoch { return Err(Error::SessionChanged); }
                state.devices.request += 1;
                state.devices.list = None;
                state.devices.request
            };
            self.changed();
            let inventory = self.request_for_session(epoch,
                AccountRequest::new(Method::Get, "/v1/devices", None)?).await?;
            self.current_device_request(epoch, sequence)?;
            inventory.clone().success()?;
            self.publish_devices(epoch, sequence, inventory.body, &current_device)?;
        } else {
            self.publish_devices(epoch, sequence, response.body.clone(), &current_device)?;
        }
        Ok(response)
    }

    fn current_device_request(&self, epoch: u64, sequence: u64) -> Result<()> {
        let state = self.state();
        if state.epoch == epoch && state.devices.request == sequence { Ok(()) }
        else { Err(Error::SessionChanged) }
    }

    fn publish_devices(&self, epoch: u64, sequence: u64, mut list: Value, current: &str) -> Result<()> {
        let limit = list["deviceLimit"].as_u64().filter(|limit| *limit > 0 && *limit <= u32::MAX as u64)
            .ok_or(Error::InvalidResponse)?;
        let devices = list["devices"].as_array_mut().ok_or(Error::InvalidResponse)?;
        if devices.len() > 1_000 { return Err(Error::InvalidResponse); }
        let mut ids = std::collections::HashSet::new();
        for device in devices {
            let id = device["id"].as_str().filter(|id| crate::request::uuid(id)).ok_or(Error::InvalidResponse)?.to_owned();
            if !ids.insert(id.clone()) || ["platform", "displayName", "activatedAt", "lastSeenAt"].iter()
                .any(|key| !device[key].is_string()) { return Err(Error::InvalidResponse); }
            device["current"] = Value::Bool(id == current);
        }
        list["deviceLimit"] = Value::from(limit);
        {
            let mut state = self.mutate();
            if state.epoch != epoch || state.devices.request != sequence { return Err(Error::SessionChanged); }
            state.devices.list = Some(list);
        }
        self.changed();
        Ok(())
    }
}
