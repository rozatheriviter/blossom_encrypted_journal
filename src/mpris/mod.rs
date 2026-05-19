use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct NowPlaying {
    pub _player: String,
    pub title: String,
    pub artist: String,
    pub _album: String,
    pub status: PlaybackStatus,
    pub _can_play: bool,
    pub _can_next: bool,
    pub _can_prev: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum PlaybackStatus {
    #[default]
    Stopped,
    Playing,
    Paused,
}

impl PlaybackStatus {
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Playing => "media-playback-pause-symbolic",
            _ => "media-playback-start-symbolic",
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Background watcher
// ──────────────────────────────────────────────────────────────────────────────

pub struct MprisWatcher {
    pub state: Arc<Mutex<Option<NowPlaying>>>,
    stop: Arc<std::sync::atomic::AtomicBool>,
    _thread: std::thread::JoinHandle<()>,
}

impl MprisWatcher {
    pub fn start() -> Self {
        let state = Arc::new(Mutex::new(None::<NowPlaying>));
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));

        let st = Arc::clone(&state);
        let sp = Arc::clone(&stop);
        let thread = std::thread::spawn(move || {
            async_io::block_on(async move {
                loop {
                    if sp.load(std::sync::atomic::Ordering::Relaxed) { break; }
                    match poll().await {
                        Ok(np) => *st.lock().unwrap() = np,
                        Err(_) => *st.lock().unwrap() = None,
                    }
                    async_io::Timer::after(Duration::from_secs(2)).await;
                }
            });
        });

        MprisWatcher { state, stop, _thread: thread }
    }

    pub fn get(&self) -> Option<NowPlaying> {
        self.state.lock().unwrap().clone()
    }
}

impl Drop for MprisWatcher {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// D-Bus helpers (zbus async-io)
// ──────────────────────────────────────────────────────────────────────────────

async fn poll() -> zbus::Result<Option<NowPlaying>> {
    let conn = zbus::Connection::session().await?;
    let dbus = zbus::fdo::DBusProxy::new(&conn).await?;
    let names = dbus.list_names().await?;

    // Convert names to String early to avoid lifetime/deref complexity
    let bus_name: Option<String> = names
        .iter()
        .map(|n| n.to_string())
        .find(|n| n.starts_with("org.mpris.MediaPlayer2."));

    let Some(bus_name) = bus_name else { return Ok(None); };

    let proxy = zbus::Proxy::new(
        &conn,
        bus_name.as_str(),
        "/org/mpris/MediaPlayer2",
        "org.mpris.MediaPlayer2.Player",
    )
    .await?;

    let metadata: std::collections::HashMap<String, zbus::zvariant::OwnedValue> =
        proxy.get_property("Metadata").await?;

    let status_str: String = proxy.get_property("PlaybackStatus").await.unwrap_or_default();
    let can_play: bool = proxy.get_property("CanPlay").await.unwrap_or(false);
    let can_next: bool = proxy.get_property("CanGoNext").await.unwrap_or(false);
    let can_prev: bool = proxy.get_property("CanGoPrevious").await.unwrap_or(false);

    let title  = str_meta(&metadata, "xesam:title");
    let artist = arr_str_meta(&metadata, "xesam:artist");
    let album  = str_meta(&metadata, "xesam:album");

    let status = match status_str.as_str() {
        "Playing" => PlaybackStatus::Playing,
        "Paused"  => PlaybackStatus::Paused,
        _         => PlaybackStatus::Stopped,
    };

    let player = bus_name
        .strip_prefix("org.mpris.MediaPlayer2.")
        .unwrap_or(bus_name.as_str())
        .to_owned();

    Ok(Some(NowPlaying { _player: player, title, artist, _album: album, status, _can_play: can_play, _can_next: can_next, _can_prev: can_prev }))
}

fn str_meta(
    map: &std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
    key: &str,
) -> String {
    use zbus::zvariant::Value;
    map.get(key)
        .and_then(|v| match &**v {
            Value::Str(s) => Some(s.as_str().to_owned()),
            _ => None,
        })
        .unwrap_or_default()
}

fn arr_str_meta(
    map: &std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
    key: &str,
) -> String {
    use zbus::zvariant::Value;
    map.get(key)
        .and_then(|v| match &**v {
            Value::Array(arr) => arr.iter().find_map(|item| match item {
                Value::Str(s) => Some(s.as_str().to_owned()),
                _ => None,
            }),
            _ => None,
        })
        .unwrap_or_default()
}

// ──────────────────────────────────────────────────────────────────────────────
// Transport commands (fire-and-forget from GTK callbacks)
// ──────────────────────────────────────────────────────────────────────────────

pub fn send_command(method: &'static str) {
    std::thread::spawn(move || {
        async_io::block_on(async move {
            let _ = do_send(method).await;
        });
    });
}

async fn do_send(method: &str) -> zbus::Result<()> {
    let conn = zbus::Connection::session().await?;
    let dbus = zbus::fdo::DBusProxy::new(&conn).await?;
    let names = dbus.list_names().await?;

    let bus_name: Option<String> = names
        .iter()
        .map(|n| n.to_string())
        .find(|n| n.starts_with("org.mpris.MediaPlayer2."));

    let Some(bus_name) = bus_name else { return Ok(()); };

    let proxy = zbus::Proxy::new(
        &conn,
        bus_name.as_str(),
        "/org/mpris/MediaPlayer2",
        "org.mpris.MediaPlayer2.Player",
    )
    .await?;

    let _: () = proxy.call_method(method, &()).await?.body().deserialize()?;
    Ok(())
}
