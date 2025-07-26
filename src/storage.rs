use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
mod web {
    use gloo::storage::Storage;
    use serde::{Deserialize, Serialize};

    pub fn set<T: Serialize>(key: impl AsRef<str>, value: T) -> Result<(), String> {
        gloo::storage::LocalStorage::set(key, value).map_err(|e| e.to_string())
    }

    pub fn get<T: for<'de> Deserialize<'de>>(key: impl AsRef<str>) -> Result<T, String> {
        gloo::storage::LocalStorage::get(key).map_err(|e| e.to_string())
    }

    pub fn try_get<T: for<'de> Deserialize<'de>>(
        key: impl AsRef<str>,
    ) -> Result<Option<T>, String> {
        match gloo::storage::LocalStorage::get(key) {
            Ok(value) => Ok(Some(value)),
            Err(gloo::storage::errors::StorageError::KeyNotFound(_)) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn remove(key: impl AsRef<str>) -> Result<(), String> {
        Ok(gloo::storage::LocalStorage::delete(key))
    }
}

#[cfg(not(feature = "web"))]
mod fs {
    use directories::ProjectDirs;
    use serde::{Deserialize, Serialize};
    use std::{io::Write, path::PathBuf};

    pub fn get_data_dir() -> Option<PathBuf> {
        ProjectDirs::from("com", "meeshroom", "tak").map(|dirs| dirs.data_dir().to_path_buf())
    }

    pub fn set<T: Serialize>(key: impl AsRef<str>, value: T) -> Result<(), String> {
        let file_path = get_data_dir()
            .map(|dir| dir.join(key.as_ref()))
            .ok_or("Failed to get data dir")?;

        if let Some(parent) = file_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
        }

        let mut file = std::fs::File::create(file_path).map_err(|e| e.to_string())?;
        file.write_all(
            serde_json::to_string(&value)
                .map_err(|e| e.to_string())?
                .as_bytes(),
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get<T: for<'de> Deserialize<'de>>(key: impl AsRef<str>) -> Result<T, String> {
        let file_path = get_data_dir()
            .map(|dir| dir.join(key.as_ref()))
            .ok_or("Failed to get data dir")?;
        let data = std::fs::read_to_string(file_path).map_err(|e| e.to_string())?;
        serde_json::from_str(&data).map_err(|e| e.to_string())
    }

    pub fn try_get<T: for<'de> Deserialize<'de>>(
        key: impl AsRef<str>,
    ) -> Result<Option<T>, String> {
        let file_path = get_data_dir()
            .map(|dir| dir.join(key.as_ref()))
            .ok_or("Failed to get data dir")?;
        if !file_path.exists() {
            return Ok(None);
        }
        let data = std::fs::read_to_string(file_path).map_err(|e| e.to_string())?;
        serde_json::from_str(&data).map_err(|e| e.to_string())
    }

    pub fn remove(key: impl AsRef<str>) -> Result<(), String> {
        let file_path = get_data_dir()
            .map(|dir| dir.join(key.as_ref()))
            .ok_or("Failed to get data dir")?;
        std::fs::remove_file(file_path).map_err(|e| e.to_string())
    }
}

pub fn set<T: Serialize>(key: impl AsRef<str>, value: T) -> Result<(), String> {
    #[cfg(feature = "web")]
    return web::set(key, value);

    #[cfg(not(feature = "web"))]
    return fs::set(key, value);
}

pub fn get<T: for<'de> Deserialize<'de>>(key: impl AsRef<str>) -> Result<T, String> {
    #[cfg(feature = "web")]
    return web::get(key);

    #[cfg(not(feature = "web"))]
    return fs::get(key);
}

pub fn try_get<T: for<'de> Deserialize<'de>>(key: impl AsRef<str>) -> Result<Option<T>, String> {
    #[cfg(feature = "web")]
    return web::try_get(key);

    #[cfg(not(feature = "web"))]
    return fs::try_get(key);
}

pub fn remove(key: impl AsRef<str>) -> Result<(), String> {
    #[cfg(feature = "web")]
    return web::remove(key);

    #[cfg(not(feature = "web"))]
    return fs::remove(key);
}
