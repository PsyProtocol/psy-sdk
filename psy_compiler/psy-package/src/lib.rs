pub mod errors;
pub mod files;
pub mod fm;
mod git;
pub mod package;
mod semver;
pub mod workspace;

pub use errors::{ManifestError, SemverError};
pub use files::*;
pub use package::*;
// Individual re-exports for backward compatibility
pub use package::{CrateName, Dependency, Package, PackageType};
pub use workspace::{Workspace, *};

pub const FILE_EXTENSION: &str = "psy";

// Re-exports for backward compatibility
pub mod manifest {
    pub use crate::{
        errors::{ManifestError, SemverError},
        resolve_workspace_from_toml, try_clone_std,
    };
}

// Main functionality - internal imports
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::{fm::NormalizePath, git::clone_git_repo};

#[derive(Debug, Deserialize, Clone)]
struct PackageConfig {
    package: PackageMetadata,
    #[serde(default)]
    dependencies: BTreeMap<String, DependencyConfig>,
}

const STD_GIT_PATH_HTTPS: &str = "https://github.com/PsyProtocol/psy-v1";
const STD_GIT_PATH_SSH: &str = "git@github.com:PsyProtocol/psy-v1.git";
const STD_TAG: &str = "v0.0.1-rc";
const TAG_LATEST: &str = "latest";
const STD_FILE: &str = "psy_compiler/psy-std/std.psy";

impl PackageConfig {
    fn resolve_to_package(&self, root_dir: &Path, processed: &mut Vec<String>) -> Result<crate::package::Package, ManifestError> {
        let name: crate::package::CrateName = if let Some(name) = &self.package.name {
            name.parse().map_err(|_| ManifestError::InvalidPackageName {
                toml: root_dir.join("Dargo.toml"),
                name: name.into(),
            })?
        } else {
            return Err(ManifestError::MissingNameField {
                toml: root_dir.join("Dargo.toml"),
            });
        };

        if std::env::var("DARGO_STD_PATH").is_err() {
            let psy_path = try_clone_std("feature/psy-lang-realms")?;
            unsafe {
                std::env::set_var("DARGO_STD_PATH", psy_path.join(STD_FILE));
            }
        }

        let mut dependencies: BTreeMap<crate::package::CrateName, crate::package::Dependency> = BTreeMap::new();
        for (name, dep_config) in self.dependencies.iter() {
            let name = name.parse().map_err(|_| ManifestError::InvalidDependencyName {
                toml: root_dir.join("Dargo.toml"),
                name: name.into(),
            })?;
            let resolved_dep = dep_config.resolve_to_dependency(root_dir, processed)?;
            dependencies.insert(name, resolved_dep);
        }

        let package_type = match self.package.package_type.as_deref() {
            Some("lib") => crate::package::PackageType::Library,
            Some("bin") => crate::package::PackageType::Binary,
            Some(invalid) => {
                return Err(ManifestError::InvalidPackageType(root_dir.join("Dargo.toml"), invalid.to_string()));
            }
            None => {
                return Err(ManifestError::MissingPackageType(root_dir.join("Dargo.toml")));
            }
        };

        let entry_path = if let Some(entry_path) = &self.package.entry {
            let custom_entry_path = root_dir.join(entry_path);
            custom_entry_path
        } else {
            let default_entry_path = match package_type {
                crate::package::PackageType::Library => root_dir.join("src").join("lib").with_extension(FILE_EXTENSION),
                crate::package::PackageType::Binary => root_dir.join("src").join("main").with_extension(FILE_EXTENSION),
            };
            default_entry_path
        };

        if let Some(version) = &self.package.version {
            semver::parse_semver_compatible_version(version).map_err(|err| {
                ManifestError::SemverError(SemverError::CouldNotParsePackageVersion {
                    package_name: name.to_string(),
                    error: err.to_string(),
                })
            })?;
        }

        Ok(crate::package::Package {
            version: self.package.version.clone(),
            root_dir: root_dir.to_path_buf(),
            entry_path,
            package_type,
            name,
            dependencies,
        })
    }
}

pub fn try_clone_std(tag: &str) -> Result<PathBuf, ManifestError> {
    match clone_git_repo(STD_GIT_PATH_HTTPS, tag) {
        Ok(path) => return Ok(path),
        Err(e) => eprintln!("[Psy] HTTPS clone failed: {}", e),
    }

    match clone_git_repo(STD_GIT_PATH_SSH, tag) {
        Ok(path) => Ok(path),
        Err(e) => {
            eprintln!("[Psy] SSH clone failed: {}", e);
            Err(ManifestError::GitError(format!(
                "Both HTTPS and SSH clone failed for psy-std (tag = {})",
                tag
            )))
        }
    }
}

#[derive(Default, Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct PackageMetadata {
    name: Option<String>,
    version: Option<String>,
    #[serde(alias = "type")]
    package_type: Option<String>,
    entry: Option<PathBuf>,
    description: Option<String>,
    authors: Option<Vec<String>>,
    license: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
enum DependencyConfig {
    Github { git: String, tag: String, directory: Option<String> },
    Path { path: String },
}

impl DependencyConfig {
    fn resolve_to_dependency(&self, pkg_root: &Path, processed: &mut Vec<String>) -> Result<crate::package::Dependency, ManifestError> {
        let dep = match self {
            Self::Github { git, tag, directory } => {
                let dir_path = clone_git_repo(git, tag).map_err(ManifestError::GitError)?;
                let project_path = if let Some(directory) = directory {
                    let internal_path = dir_path.join(directory).normalize();
                    if !internal_path.starts_with(&dir_path) {
                        return Err(ManifestError::InvalidDirectory {
                            toml: pkg_root.join("Dargo.toml"),
                            directory: directory.into(),
                        });
                    }
                    internal_path
                } else {
                    dir_path
                };
                let toml_path = project_path.join("Dargo.toml");
                let package = resolve_package_from_toml(&toml_path, processed)?;
                crate::package::Dependency::Remote { package }
            }
            Self::Path { path } => {
                let dir_path = pkg_root.join(path);
                let toml_path = dir_path.join("Dargo.toml");
                let package = resolve_package_from_toml(&toml_path, processed)?;
                crate::package::Dependency::Local { package }
            }
        };
        Ok(dep)
    }
}

/// Resolves a Dargo.toml file into a `Workspace` struct.
pub fn resolve_workspace_from_toml(toml_path: &Path) -> Result<crate::workspace::Workspace, ManifestError> {
    let dargo_toml = read_toml(toml_path)?;
    let mut resolved = Vec::new();
    let workspace = match dargo_toml.config {
        Config::Package { package_config } => {
            let member = package_config.resolve_to_package(&dargo_toml.root_dir, &mut resolved)?;
            let target_dir = dargo_toml.root_dir.join("target").normalize();
            crate::workspace::Workspace {
                root_dir: dargo_toml.root_dir,
                target_dir,
                package: member,
            }
        }
    };
    Ok(workspace)
}

fn resolve_package_from_toml(toml_path: &Path, processed: &mut Vec<String>) -> Result<crate::package::Package, ManifestError> {
    let str_path = toml_path.to_str().expect("ICE - path is empty");
    if processed.contains(&str_path.to_string()) {
        let mut cycle = false;
        let mut message = String::new();
        for toml in processed {
            cycle = cycle || toml == str_path;
            if cycle {
                message += &format!("{} referencing ", toml);
            }
        }
        message += str_path;
        return Err(ManifestError::CyclicDependency { cycle: message });
    }

    if let Some(str) = toml_path.to_str() {
        processed.push(str.to_string());
    }

    let dargo_toml = read_toml(toml_path)?;
    let result = match dargo_toml.config {
        Config::Package { package_config } => package_config.resolve_to_package(&dargo_toml.root_dir, processed),
    };
    let pos = processed.iter().position(|toml| toml == str_path).expect("added package must be here");
    processed.remove(pos);
    result
}

struct DargoToml {
    root_dir: PathBuf,
    config: Config,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
enum Config {
    Package {
        #[serde(flatten)]
        package_config: PackageConfig,
    },
}

impl TryFrom<String> for Config {
    type Error = toml::de::Error;

    fn try_from(toml: String) -> Result<Self, Self::Error> {
        toml::from_str(&toml)
    }
}

impl TryFrom<&str> for Config {
    type Error = toml::de::Error;

    fn try_from(toml: &str) -> Result<Self, Self::Error> {
        toml::from_str(toml)
    }
}

fn read_toml(toml_path: &Path) -> Result<DargoToml, ManifestError> {
    let toml_path = toml_path.normalize();
    let toml_as_string = std::fs::read_to_string(&toml_path).map_err(|_| ManifestError::ReadFailed(toml_path.to_path_buf()))?;
    let root_dir = toml_path.parent().ok_or(ManifestError::MissingParent)?;
    let dargo_toml = DargoToml {
        root_dir: root_dir.to_path_buf(),
        config: toml_as_string.try_into()?,
    };
    Ok(dargo_toml)
}
