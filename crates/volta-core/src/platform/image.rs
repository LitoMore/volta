use std::ffi::OsString;
use std::path::PathBuf;

use super::{build_path_error, Sourced};
use crate::error::{Context, Fallible};
#[cfg(not(feature = "package-global"))]
use crate::layout::env_paths;
use crate::layout::volta_home;
use crate::tool::load_default_npm_version;
use semver::Version;

/// A platform image.
pub struct Image {
    /// The pinned version of Node.
    pub node: Sourced<Version>,
    /// The custom version of npm, if any. `None` represents using the npm that is bundled with Node
    pub npm: Option<Sourced<Version>>,
    /// The pinned version of Yarn, if any.
    pub yarn: Option<Sourced<Version>>,
}

impl Image {
    fn bins(&self) -> Fallible<Vec<PathBuf>> {
        let home = volta_home()?;
        let mut bins = Vec::with_capacity(3);

        if let Some(npm) = &self.npm {
            let npm_str = npm.value.to_string();
            bins.push(home.npm_image_bin_dir(&npm_str));
        }

        if let Some(yarn) = &self.yarn {
            let yarn_str = yarn.value.to_string();
            bins.push(home.yarn_image_bin_dir(&yarn_str));
        }

        // Add Node path to the bins last, so that any custom version of npm will be earlier in the PATH
        let node_str = self.node.value.to_string();
        bins.push(home.node_image_bin_dir(&node_str));
        Ok(bins)
    }

    /// Produces a modified version of the current `PATH` environment variable that
    /// will find toolchain executables (Node, Yarn) in the installation directories
    /// for the given versions instead of in the Volta shim directory.
    pub fn path(&self) -> Fallible<OsString> {
        let old_path = envoy::path().unwrap_or_else(|| envoy::Var::from(""));

        #[cfg(not(feature = "package-global"))]
        {
            let mut new_path = old_path.split();

            for remove_path in env_paths()? {
                new_path = new_path.remove(remove_path);
            }

            new_path
                .prefix(self.bins()?)
                .join()
                .with_context(build_path_error)
        }

        #[cfg(feature = "package-global")]
        {
            old_path
                .split()
                .prefix(self.bins()?)
                .join()
                .with_context(build_path_error)
        }
    }

    /// Determines the sourced version of npm that will be available, resolving the version bundled with Node, if needed
    pub fn resolve_npm(&self) -> Fallible<Sourced<Version>> {
        match &self.npm {
            Some(npm) => Ok(npm.clone()),
            None => load_default_npm_version(&self.node.value).map(|npm| Sourced {
                value: npm,
                source: self.node.source,
            }),
        }
    }
}
