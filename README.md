# TontooData

Data storage library for TontooOS with sandboxing and system hooks.

## Made for TontooOS

Explore more at https://github.com/arlomu/TontooOSLibs

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
tontoo-data = { git = "https://github.com/arlomu/TontooData" }
```

For encryption support:

```toml
[dependencies]
tontoo-data = { git = "https://github.com/arlomu/TontooData", features = ["encryption"] }
```

## Features

- **SQLite Backend**: Efficient key-value storage with lazy loading
- **App Sandboxing**: Isolated per application (separate database per app)
- **Optional Encryption**: AES-256-GCM when `encryption` feature is enabled
- **System Hooks**: Special API for Settings app to manage storage sizes
- **Cross-platform**: Works on Linux (primary target for TontooOS)

## For Applications

```rust
use tontoo_data::TontooData;

// Set environment variable: TONTOO_APP_ID=com.example.myapp
let data = TontooData::init()?;
data.save("settings", &my_settings)?;
let settings: Settings = data.load("settings")?.unwrap();
```

## For System Apps (Settings)

```rust
use tontoo_data::SystemDataAccess;

// Set environment variable: TONTOO_SYSTEM_APP=true
let system = SystemDataAccess::init()?;
let apps = system.list_all_apps()?;
for app in apps {
    println!("{}: {} bytes", app.id, app.total_size);
}
system.delete_app_data("com.example.myapp")?;
```

## Environment Variables

- `TONTOO_APP_ID` - Application identifier for sandboxing
- `TONTOO_SYSTEM_APP=true` - Enables system management access

## Repository

https://github.com/arlomu/TontooData

## License

MIT