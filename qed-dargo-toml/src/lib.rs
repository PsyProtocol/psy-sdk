pub mod errors;
pub mod files;
mod git;
mod semver;

use crate::errors::{ManifestError, SemverError};
use crate::git::clone_git_repo;
use qed_dargo::fm::NormalizePath;
use qed_dargo::package::{CrateName, Dependency, Package, PackageType};
use qed_dargo::workspace::Workspace;
use qed_dargo::FILE_EXTENSION;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Clone)]
struct PackageConfig {
    package: PackageMetadata,
    #[serde(default)]
    dependencies: BTreeMap<String, DependencyConfig>,
}

const STD_GIT_PATH: &str = "https://github.com/QEDProtocol/qed-lang";
const STD_TAG: &str = "v0.0.1-rc";
const STD_DIR: &str = "qed-std/std.qed";

impl PackageConfig {
    fn resolve_to_package(
        &self,
        root_dir: &Path,
        processed: &mut Vec<String>,
    ) -> Result<Package, ManifestError> {
        let name: CrateName = if let Some(name) = &self.package.name {
            name.parse()
                .map_err(|_| ManifestError::InvalidPackageName {
                    toml: root_dir.join("Dargo.toml"),
                    name: name.into(),
                })?
        } else {
            return Err(ManifestError::MissingNameField {
                toml: root_dir.join("Dargo.toml"),
            });
        };

        if std::env::var("DARGO_STD_PATH").is_err() {
            let qed_path =
                clone_git_repo(STD_GIT_PATH, STD_TAG).map_err(ManifestError::GitError)?;
            unsafe { std::env::set_var("DARGO_STD_PATH", qed_path.join(STD_DIR)); }
        }

        let mut dependencies: BTreeMap<CrateName, Dependency> = BTreeMap::new();
        for (name, dep_config) in self.dependencies.iter() {
            let name = name
                .parse()
                .map_err(|_| ManifestError::InvalidDependencyName {
                    toml: root_dir.join("Dargo.toml"),
                    name: name.into(),
                })?;
            let resolved_dep = dep_config.resolve_to_dependency(root_dir, processed)?;

            dependencies.insert(name, resolved_dep);
        }

        let package_type = match self.package.package_type.as_deref() {
            Some("lib") => PackageType::Library,
            Some("bin") => PackageType::Binary,
            Some(invalid) => {
                return Err(ManifestError::InvalidPackageType(
                    root_dir.join("Dargo.toml"),
                    invalid.to_string(),
                ));
            }
            None => {
                return Err(ManifestError::MissingPackageType(
                    root_dir.join("Dargo.toml"),
                ));
            }
        };

        let entry_path = if let Some(entry_path) = &self.package.entry {
            let custom_entry_path = root_dir.join(entry_path);
            custom_entry_path
        } else {
            let default_entry_path = match package_type {
                PackageType::Library => root_dir
                    .join("src")
                    .join("lib")
                    .with_extension(FILE_EXTENSION),
                PackageType::Binary => root_dir
                    .join("src")
                    .join("main")
                    .with_extension(FILE_EXTENSION),
            };
            default_entry_path
        };

        // If there is a package version, ensure that it is semver compatible
        if let Some(version) = &self.package.version {
            semver::parse_semver_compatible_version(version).map_err(|err| {
                ManifestError::SemverError(SemverError::CouldNotParsePackageVersion {
                    package_name: name.to_string(),
                    error: err.to_string(),
                })
            })?;
        }

        Ok(Package {
            version: self.package.version.clone(),
            root_dir: root_dir.to_path_buf(),
            entry_path,
            package_type,
            name,
            dependencies,
        })
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
/// Enum representing the different types of ways to
/// supply a source for the dependency
enum DependencyConfig {
    Github {
        git: String,
        tag: String,
        directory: Option<String>,
    },
    Path {
        path: String,
    },
}

impl DependencyConfig {
    fn resolve_to_dependency(
        &self,
        pkg_root: &Path,
        processed: &mut Vec<String>,
    ) -> Result<Dependency, ManifestError> {
        let dep = match self {
            Self::Github {
                git,
                tag,
                directory,
            } => {
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
                Dependency::Remote { package }
            }
            Self::Path { path } => {
                let dir_path = pkg_root.join(path);
                let toml_path = dir_path.join("Dargo.toml");
                let package = resolve_package_from_toml(&toml_path, processed)?;
                Dependency::Local { package }
            }
        };

        Ok(dep)
    }
}

/// Resolves a Dargo.toml file into a `Workspace` struct.
///
/// As a side effect it downloads project dependencies as well.
pub fn resolve_workspace_from_toml(toml_path: &Path) -> Result<Workspace, ManifestError> {
    let dargo_toml = read_toml(toml_path)?;
    let mut resolved = Vec::new();
    let workspace = match dargo_toml.config {
        Config::Package { package_config } => {
            let member = package_config.resolve_to_package(&dargo_toml.root_dir, &mut resolved)?;
            let target_dir = dargo_toml.root_dir.join("target").normalize();
            Workspace {
                root_dir: dargo_toml.root_dir,
                target_dir,
                package: member,
            }
        }
    };
    Ok(workspace)
}

/// Resolves a Dargo.toml file into a `Package` struct as defined by our `dargo` core.
fn resolve_package_from_toml(
    toml_path: &Path,
    processed: &mut Vec<String>,
) -> Result<Package, ManifestError> {
    // Checks for cyclic dependencies
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
    // Adds the package to the set of resolved packages
    if let Some(str) = toml_path.to_str() {
        processed.push(str.to_string());
    }

    let dargo_toml = read_toml(toml_path)?;

    let result = match dargo_toml.config {
        Config::Package { package_config } => {
            package_config.resolve_to_package(&dargo_toml.root_dir, processed)
        }
    };
    let pos = processed
        .iter()
        .position(|toml| toml == str_path)
        .expect("added package must be here");
    processed.remove(pos);
    result
}

struct DargoToml {
    root_dir: PathBuf,
    config: Config,
}

/// Contains all the information about a package, as loaded from a `Dargo.toml`.
///
/// This type can be extended in the future to support workspace.
#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
enum Config {
    /// Represents a `Dargo.toml` with package fields.
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
    let toml_as_string = std::fs::read_to_string(&toml_path)
        .map_err(|_| ManifestError::ReadFailed(toml_path.to_path_buf()))?;
    let root_dir = toml_path.parent().ok_or(ManifestError::MissingParent)?;
    let dargo_toml = DargoToml {
        root_dir: root_dir.to_path_buf(),
        config: toml_as_string.try_into()?,
    };
    Ok(dargo_toml)
}

#[cfg(test)]
mod tests {
    use super::Config;
    #[test]
    fn parse_standard_toml() {
        let src = r#"

        [package]
        name = "test"
        version = "0.1.0"
        type = "lib"
        authors = ["foo", "foo"]

        [dependencies]
        rand = { tag = "next", git = "https://github.com/rust-lang-nursery/rand"}
        cool = { tag = "next", git = "https://github.com/rust-lang-nursery/rand"}
        hello = {path = "./hello_world"}
    "#;

        assert!(Config::try_from(String::from(src)).is_ok());
        assert!(Config::try_from(src).is_ok());
    }

    #[test]
    fn parse_package_toml_no_deps() {
        let src = r#"
        [package]
        name = "test"
        version = "0.1.0"
        type = "lib"
        authors = ["foo", "foo"]
    "#;

        assert!(Config::try_from(String::from(src)).is_ok());
        assert!(Config::try_from(src).is_ok());
    }
}
