# TontooOS Data Library - Rust Implementation Guide

## Überblick

Diese Data Library für TontooOS im macOS-Style bietet isolierte App-Datenspeicherung mit speziellen System-Hook-Funktionen für das Management von Daten-Größen und -Löschung.

## Architektur-Empfehlungen

### 1. Grundlegende Komponenten

```
tontoo-data/
├── src/
│   ├── lib.rs              # Hauptbibliothek
│   ├── storage.rs          # Speicher-Engine (Linux Backend)
│   ├── sandbox.rs          # App-Sandboxing & Isolation
│   ├── system_hooks.rs     # Spezielle Hooks für System-Apps
│   ├── quota.rs           # Speichergrößen-Management
│   ├── permissions.rs       # Berechtigungs-System
│   └── cleanup.rs         # Datenbereinigung
├── Cargo.toml
└── tontoo-data-service/   # Optionaler Hintergrunddienst
    ├── src/
    │   └── daemon.rs
    └── Cargo.toml
```

### 2. App-Isolation (Sandboxing)

**Prinzip:** Jede App hat einen eindeutigen Identifier, der als Namespace für Daten verwendet wird.

```rust
// Beispiel-Implementierung
pub struct AppSandbox {
    app_id: String,
    data_root: PathBuf,
}

impl AppSandbox {
    pub fn new(app_id: &str) -> Result<Self> {
        let data_root = dirs::data_local_dir()
            .join("TontooOS")
            .join("Apps")
            .join(app_id);
        Ok(Self { app_id, data_root })
    }
}
```

**Speicherorte:**
- Nutzer-Daten: `~/.local/share/tontoo/apps/{app_id}/`
- System-Daten: `/var/lib/tontoo/system/`
- Cache: `~/.cache/tontoo/{app_id}/`

### 3. Speicher-Backend (Linux)

**Option A: Direkt auf Filesystem (Empfohlen)**
- Kein separater Dienst erforderlich
- Jeder App-Prozess schreibt direkt in sein isoliertes Verzeichnis
- Dateisystem-Berechtigungen durch Standard-Linux-Rechte

**Option B: Hintergrunddienst (für erweiterte Features)**
Verwende diesen Ansicht, wenn du:
- Zentrale Quotenverwaltung brauchst
- Echtzeit-Statistiken für System-Preferences
- Cross-App-Daten-Synchronisation benötigst

### 4. System-Hooks für Settings-App

Die Settings-App benötigt spezielle Berechtigungen und Schnittstellen:

#### 4.1 Speichergrößen-Abfrage

```rust
// API für System-Preferences
pub trait SystemDataManager {
    fn get_app_storage_size(&self, app_id: &str) -> Result<u64>;
    fn get_app_cache_size(&self, app_id: &str) -> Result<u64>;
    fn list_all_apps(&self) -> Result<Vec<AppInfo>>;
}

pub struct AppInfo {
    pub id: String,
    pub name: String,
    pub data_size: u64,
    pub cache_size: u64,
    pub last_access: std::time::SystemTime,
}
```

#### 4.2 Daten-Löschung mit Hooks

```rust
pub trait DataCleanup {
    /// Löscht alle Daten einer App (von System-Preferences aufgerufen)
    fn delete_app_data(&self, app_id: &str) -> Result<()>;
    
    /// Löscht Cache-Daten (freiwillig durch Benutzer)
    fn clear_cache(&self, app_id: &str) -> Result<()>;
    
    /// App-spezifischer Cleanup-Hook
    fn register_cleanup_handler(&self, app_id: &str, callback: fn()) -> Result<()>;
}
```

### 5. Daten-Management API

```rust
// Öffentliche API der Library
pub struct TontooData {
    sandbox: AppSandbox,
}

impl TontooData {
    /// Initialisiert die Datenbibliothek für eine App
    pub fn init(app_id: &str) -> Result<Self>;
    
    /// Speichert Daten (serialisierbar mit Serde)
    pub fn save<T: Serialize>(&self, key: &str, data: &T) -> Result<()>;
    
    /// Lädt Daten
    pub fn load<T: Deserialize>(&self, key: &str) -> Result<Option<T>>;
    
    /// Speicherort für große Dateien
    pub fn file_path(&self, filename: &str) -> PathBuf;
}

// Für System-Apps (wie Settings)
pub struct SystemDataAccess {
    // Erweiterte Rechte
}
```

### 6. Implementierungsstrategie

#### Phase 1: Basis-Library
1. **App-Sandboxing**: Einfaches Verzeichnis-Management pro App
2. **Serializer**: Serde für JSON/Binär-Serialisierung
3. **Basic CRUD**: save/load/delete Operationen

#### Phase 2: System-Integration
1. **System-Token**: Umgebungsvariable oder Socket für System-Apps
2. **Größen-Berechnung**: Rekursives Durchsuchen der App-Daten
3. **Cleanup-Funktionen**: App-Daten löschen mit Bestätigung

#### Phase 3: Optionaler Hintergrunddienst
```rust
// tontoo-data-service als separater Binärbaum
// Kommuniziert über D-Bus oder Unix-Socket
// Bietet:
// - Zentrale Statistiken
// - Hintergrund-Bereinigung
// - Quoten-Enforcement
```

### 7. Sicherheitskonzept

**Dateisystem-Berechtigungen:**
- Jede App läuft unter einem eindeutigen User oder nutzt Capabilities
- Datenverzeichnisse gehören der App (700-Rechte)
- System-Apps haben zusätzliche Capabilities via Capabilities-System

**Capability-basiertes Rechte-System:**
```rust
// capabilities.toml (im App-Bundle)
[permissions]
storage = true
cache = true
system_data_access = false  # Nur für System-Apps

[SystemPreferences]
system_data_access = true
app_management = true
```

### 8. Beispiel-Verwendung

**Für normale Apps:**
```rust
use tontoo_data::TontooData;

let data = TontooData::init("com.example.myapp")?;
data.save("settings", &my_settings)?;
let settings: Settings = data.load("settings")?.unwrap();
```

**Für System-Preferences App:**
```rust
use tontoo_data::SystemDataAccess;

let system = SystemDataAccess::init()?; // Prüft System-Token
let app_info = system.get_app_storage_size("com.example.myapp")?;
system.delete_app_data("com.example.myapp")?; // Mit Bestätigung
```

### 9. Technische Details

**Abhängigkeiten (Cargo.toml):**
```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
dirs = "5"
walkdir = "2"  # Für Rekursiv-Durchsuchen
thiserror = "1"
```

**Optional für Service:**
```toml
dbus = "0.9"  # Für D-Bus Kommunikation
tokio = { version = "1", features = ["full"] }  # Async Runtime
```

### 10. Dateistruktur im Detail

```
~/.local/share/tontoo/apps/
├── com.example.app1/
│   ├── data/
│   │   ├── settings.json
│   │   └── userdata.bin
│   └── cache/
│       └── temp_files/
├── com.tontoo.systempreferences/
│   └── (erweiterte Rechte)
```

### 11. Empfehlung für System-Hooks

**Verwende eine Kombination aus:**
1. **Capability-basiertes Rechte-System** für App-Erkennung
2. **Umgebungs-Variable `TONTOO_SYSTEM_APP=true`** für System-Apps
3. **IPC via Unix-Socket** für den optionalen Hintergrunddienst

**Kein separater Dienst erforderlich** für die Basis-Features. Die Settings-App kann direkt auf die Daten zugreifen, solange sie die System-App-Berechtigung hat.

---

## Fazit

Starte mit einer einfachen Library ohne Hintergrunddienst. Implementiere das Sandboxing und die System-Hooks über Berechtigungscheck. Füge erst später einen Hintergrunddienst hinzu, wenn du Features wie zentrale Statistiken oder Cross-App-Operationen brauchst.