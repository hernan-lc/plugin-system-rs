use sd_events::{EventBus, StreamEvent};
use sd_types::{DeviceId, DeviceInfo};
use std::sync::{Arc, RwLock};
use tokio::sync::RwLock as AsyncRwLock;

pub trait Device: Send + Sync + 'static {
    fn device_info(&self) -> DeviceInfo;
    fn press_button(&self, index: usize);
    fn release_button(&self, index: usize);
    fn set_key_image(&self, index: usize, image: &[u8]) -> Result<(), String>;
    fn set_key_text(&self, index: usize, text: &str) -> Result<(), String>;
}

pub struct VirtualDevice {
    info: DeviceInfo,
    events: Arc<EventBus>,
    pressed: Arc<RwLock<Vec<bool>>>,
}

impl VirtualDevice {
    pub fn new(id: impl Into<String>, key_count: usize, events: Arc<EventBus>) -> Self {
        Self {
            info: DeviceInfo {
                id: DeviceId(id.into()),
                name: "Virtual StreamDeck".to_string(),
                key_count,
                is_virtual: true,
            },
            events,
            pressed: Arc::new(RwLock::new(vec![false; key_count])),
        }
    }

    pub fn emit_event(&self, event: StreamEvent) {
        self.events.emit(event);
    }
}

impl Device for VirtualDevice {
    fn device_info(&self) -> DeviceInfo {
        self.info.clone()
    }

    fn press_button(&self, index: usize) {
        if index < self.info.key_count {
            let mut pressed = self.pressed.write().unwrap();
            pressed[index] = true;
            println!("[VirtualDevice] Button {} pressed", index);
        }
    }

    fn release_button(&self, index: usize) {
        if index < self.info.key_count {
            let mut pressed = self.pressed.write().unwrap();
            pressed[index] = false;
            println!("[VirtualDevice] Button {} released", index);
        }
    }

    fn set_key_image(&self, index: usize, _image: &[u8]) -> Result<(), String> {
        if index < self.info.key_count {
            Ok(())
        } else {
            Err(format!("Invalid button index: {}", index))
        }
    }

    fn set_key_text(&self, index: usize, text: &str) -> Result<(), String> {
        if index < self.info.key_count {
            println!("[VirtualDevice] Button {} text: {}", index, text);
            Ok(())
        } else {
            Err(format!("Invalid button index: {}", index))
        }
    }
}

pub struct DeviceManager {
    devices: Arc<AsyncRwLock<Vec<Arc<dyn Device>>>>,
    events: Arc<EventBus>,
}

impl DeviceManager {
    pub fn new(events: Arc<EventBus>) -> Self {
        Self {
            devices: Arc::new(AsyncRwLock::new(Vec::new())),
            events,
        }
    }

    pub async fn add_device(&self, device: Arc<dyn Device>) {
        let info = device.device_info();
        let mut devices = self.devices.write().await;
        devices.push(device);
        self.events
            .emit(StreamEvent::DeviceConnected { device: info.id });
    }

    pub async fn list_devices(&self) -> Vec<DeviceInfo> {
        let devices = self.devices.read().await;
        devices.iter().map(|d| d.device_info()).collect()
    }

    pub async fn get_device(&self, id: &DeviceId) -> Option<Arc<dyn Device>> {
        let devices = self.devices.read().await;
        devices.iter().find(|d| &d.device_info().id == id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn virtual_device_info() {
        let events = Arc::new(EventBus::new());
        let dev = VirtualDevice::new("test", 8, events);
        let info = dev.device_info();
        assert_eq!(info.id.0, "test");
        assert_eq!(info.key_count, 8);
        assert!(info.is_virtual);
    }

    #[test]
    fn virtual_device_press_release() {
        let events = Arc::new(EventBus::new());
        let dev = VirtualDevice::new("d", 4, events);
        dev.press_button(0);
        dev.release_button(0);
        assert!(dev.set_key_image(0, &[]).is_ok());
        assert!(dev.set_key_image(99, &[]).is_err());
    }

    #[test]
    fn virtual_device_set_key_text() {
        let events = Arc::new(EventBus::new());
        let dev = VirtualDevice::new("d", 2, events);
        assert!(dev.set_key_text(0, "hi").is_ok());
        assert!(dev.set_key_text(5, "oops").is_err());
    }

    #[tokio::test]
    async fn device_manager_add_and_list() {
        let events = Arc::new(EventBus::new());
        let mgr = DeviceManager::new(events.clone());
        let dev: Arc<dyn Device> = Arc::new(VirtualDevice::new("v1", 4, events));
        mgr.add_device(dev).await;
        let list = mgr.list_devices().await;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id.0, "v1");
    }

    #[tokio::test]
    async fn device_manager_get_device() {
        let events = Arc::new(EventBus::new());
        let mgr = DeviceManager::new(events.clone());
        let dev: Arc<dyn Device> = Arc::new(VirtualDevice::new("v2", 4, events));
        mgr.add_device(dev).await;
        let found = mgr.get_device(&DeviceId("v2".to_string())).await;
        assert!(found.is_some());
        assert!(mgr.get_device(&DeviceId("nope".to_string())).await.is_none());
    }
}
