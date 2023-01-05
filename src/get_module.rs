use std::path::{PathBuf};

use anyhow::{bail, Result};
use path_clean::PathClean;

use crate::apply_patch::patch_module;
use crate::archive_extractor::Extractor;
use crate::args::Install;
use crate::cache::Cache;
use crate::canon_path::CanonPath;
use crate::download::Downloader;
use crate::manifest::{Location, Module, Source, Global};
use crate::replace::ReplaceSpec;
use crate::settings::Config;

pub struct ModuleDownload<'a> {
    pub global: &'a Global,
    pub opts: &'a Install,
    pub downloader: &'a Downloader,
    pub extractor: Extractor<'a>,
    pub cache: &'a Cache,
    pub game_dir: &'a CanonPath,
}

impl <'a> ModuleDownload<'a> {

    pub fn new(config: &'a Config, global: &'a Global, opts: &'a Install, downloader: &'a Downloader,
                game_dir: &'a CanonPath, cache:&'a Cache) -> Self {
        Self {
            global,
            opts,
            downloader,
            extractor: Extractor::new(game_dir, config),
            cache,
            game_dir,
        }
    }

    // at some point, I'd like to have a pool of downloads with installations done
    // concurrently as soon as modules are there
    #[tokio::main]
    pub async fn get_module(&self, module: &Module) -> Result<()> {
        match &module.location {
            None => bail!("No location provided to retrieve missing module {}", module.name),
            Some(location) => {
                let archive = match self.retrieve_location(&location, &module).await {
                    Ok(archive) => archive,
                    Err(error) => bail!("retrieve archive failed for module {}\n-> {:?}", module.name, error),
                };

                let dest = std::env::current_dir()?;
                let dest = CanonPath::new(dest)?;
                self.extractor.extract_files(&archive, &module.name, location)?;
                patch_module(&dest, &module.name, &location.patch, &self.opts).await?;
                replace_module(&dest, &module.name, &location.replace)?;
                Ok(())
            }
        }
    }

    pub async fn retrieve_location(&self, loc: &Location, module: &Module) -> Result<PathBuf> {
        use Source::*;

        let dest = self.cache.join(loc.source.save_subdir()?);
        let save_name = loc.source.save_name(&module.name)?;
        match &loc.source {
            Http { http, .. } => self.downloader.download(http, &dest, save_name).await,
            Github(github) => github.get_github(&self.downloader, &dest, save_name).await,
            Absolute { path } => Ok(PathBuf::from(path)),
            Local { local } => self.get_local_mod_path(local),
        }
    }

    fn get_local_mod_path(&self, local_mod_name: &String) -> Result<PathBuf, anyhow::Error> {
        let manifest_path = self.get_manifest_root().clean();
        let local_mods = match &self.global.local_mods {
            None => PathBuf::new(),
            Some(path) => PathBuf::from(path).clean(),
        };
        if local_mods.is_absolute() || local_mods.starts_with("..") {
            bail!("Invalid local_mods value");
        }
        let mod_name = PathBuf::from(local_mod_name).clean();
        if mod_name.is_absolute() || local_mods.starts_with("..") {
            bail!("Invalid local value");
        }
        Ok(manifest_path.join(local_mods).join(mod_name))
    }

    pub fn get_manifest_root(&self) -> PathBuf {
        let manifest = PathBuf::from(&self.opts.manifest_path);
        match manifest.parent() {
            None => PathBuf::from(&self.game_dir),
            Some(path) => PathBuf::from(path),
        }
    }
}


fn replace_module(game_dir: &CanonPath, module_name: &str, replace: &Option<Vec<ReplaceSpec>>) -> Result<()> {
    if let Some(specs) = replace {
        for spec in specs {
            let mod_path = game_dir.join(&module_name);
            spec.exec(&mod_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod test_retrieve_location {


    use std::{path::PathBuf};

    use crate::manifest::{Location, Github, Module, Global};
    use crate::download::{Downloader};
    use crate::args::{Install};
    use crate::get_module::ModuleDownload;
    use crate:: settings::Config;
    use crate::canon_path::CanonPath;
    use crate::cache::Cache;

    use anyhow::bail;
    use faux::when;

    /**
     * Checks that for a github mod, retrieve_location(...) returns whetever is returned by download(...).
     */
    #[tokio::test]
    async fn retrieve_github_location() {
        let location = Location {
            source: crate::manifest::Source::Github(Github {
                github_user: "username".to_string(),
                repository: "repository".to_string(),
                descriptor: crate::manifest::GithubDescriptor::Release {
                    release: Some("V1".to_string()),
                    asset: "repository_v1.zip".to_string(),
                },
            }),
            ..Location::default()
        };
        let module = Module {
            location: Some(location.clone()),
            ..Module::default()
        };
        let global = Global::default();
        let opts = Install::default();
        let config = Config {
            archive_cache: Some("/cache_path".to_string()),
            extract_location: Some("/tmp".to_string())
        };

        let expected_dest = PathBuf::from("/cache_path/github/username/repository");

        let game_dir = CanonPath::new("some_dir").unwrap();
        let cache = Cache::Path(PathBuf::from("/cache_path"));

        let mut downloader = Downloader::faux();
        when!(
            downloader.download(_, {expected_dest}, _)
        ).then(|(_, _, _)| Ok(PathBuf::from("cache_dir/directory/filename.zip")));
        when!(
            downloader.download_partial(_, _, _)
        ).then(|(_, _, _)| bail!("Should not be called"));
        when!(
            downloader.rename_partial(_, _)
        ).then(|(_, _)| bail!("Should not be called"));

        let module_download = ModuleDownload::new(&config, &global, &opts,
                                                                            &downloader, &game_dir, &cache);

        let result = module_download.retrieve_location(&location, &module);
        assert_eq!(
            result.await.unwrap(),
            PathBuf::from("cache_dir/directory/filename.zip")
        )
    }

    /**
     * Check http location.
     * Should be <cache_path>/http/<host_name>/<file_name>
    * */
    #[tokio::test]
    async fn retrieve_http_location() {
        let location = Location {
            source: crate::manifest::Source::Http {
                http: "http://example.com/some_mod.zip".to_string(),
                rename: None
            },
            ..Location::default()
        };
        let module = Module {
            location: Some(location.clone()),
            ..Module::default()
        };
        let global = Global::default();
        let opts = Install::default();
        let config = Config {
            archive_cache: Some("/cache_path".to_string()),
            extract_location: Some("/tmp".to_string())
        };

        let expected_dest = PathBuf::from("/cache_path/http/example.com");

        let game_dir = CanonPath::new("some_dir").unwrap();
        let cache = Cache::Path(PathBuf::from("/cache_path"));

        let mut downloader = Downloader::faux();
        when!(
            downloader.download(_, {expected_dest}, _)
        ).then(|(_, _, _)| Ok(PathBuf::from("/cache_path/http/example.com/some_mod.zip")));
        when!(
            downloader.download_partial(_, _, _)
        ).then(|(_, _, _)| bail!("Should not be called"));
        when!(
            downloader.rename_partial(_, _)
        ).then(|(_, _)| bail!("Should not be called"));

        let module_download = ModuleDownload::new(&config, &global, &opts,
                                                                            &downloader, &game_dir, &cache);

        let result = module_download.retrieve_location(&location, &module);
        assert_eq!(
            result.await.unwrap(),
            PathBuf::from("/cache_path/http/example.com/some_mod.zip")
        )
    }

    /**
     * Check absolute location.
     * Should just be the path in the location.
     */
    #[tokio::test]
    async fn retrieve_absolute_location() {
        let location = Location {
            source: crate::manifest::Source::Absolute { path: "/some/path/file.zip".to_string() },
            ..Location::default()
        };
        let module = Module {
            location: Some(location.clone()),
            ..Module::default()
        };
        let global = Global {
            local_mods: Some("my_mods".to_string()),
            ..Default::default()
        };
        let opts = Install {
            manifest_path: "/home/me/my_install.yaml".to_string(),
            ..Install::default()
        };
        let config = Config::default();

        let game_dir = CanonPath::new("some_dir").unwrap();
        let cache = Cache::Path(PathBuf::from("/cache_path"));

        let downloader = Downloader::faux();

        let module_download = ModuleDownload::new(&config, &global, &opts,
                                                                            &downloader, &game_dir, &cache);

        let result = module_download.retrieve_location(&location, &module);
        assert_eq!(
            result.await.unwrap(),
            PathBuf::from("/some/path/file.zip")
        );
    }

    /**
     * Checks local mods.
     * Result should be <manifest_location>/<local_mods>/<mod_path>
     */
    #[tokio::test]
    async fn retrieve_local_location() {
        let location = Location {
            source: crate::manifest::Source::Local { local: "some/path/file.zip".to_string() },
            ..Location::default()
        };
        let module = Module {
            location: Some(location.clone()),
            ..Module::default()
        };
        let global = Global {
            local_mods: Some("my_mods".to_string()),
            ..Default::default()
        };
        let opts = Install {
            manifest_path: "/home/me/my_install.yaml".to_string(),
            ..Install::default()
        };
        let config = Config::default();

        let game_dir = CanonPath::new("some_dir").unwrap();
        let cache = Cache::Path(PathBuf::from("/cache_path"));

        let downloader = Downloader::faux();

        let module_download = ModuleDownload::new(&config, &global, &opts,
                                                                            &downloader, &game_dir, &cache);

        let result = module_download.retrieve_location(&location, &module);
        assert_eq!(
            result.await.unwrap(),
            PathBuf::from("/home/me/my_mods/some/path/file.zip")
        );
    }
}
