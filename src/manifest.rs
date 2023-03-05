
use std::io::{BufReader, Seek, SeekFrom};
use serde::{Deserialize, Serialize};

use anyhow::{bail, Result};
use serde_yaml::Deserializer;

use crate::global::Global;
use crate::module::module::Module;


#[derive(Deserialize, Serialize, Debug)]
pub struct VersionDetect {
    pub version: String,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct Manifest {
    /// Manifest format version
    pub version: String,
    /// Manifest-wide definitions
    pub global: Global,
    #[serde(default)]
    /// List of modules
    pub modules: Vec<Module>,
}

impl Manifest {
    pub fn read_path(path: &str) -> Result<Self> {
        let mut file = match std::fs::File::open(path) {
            Err(error) => bail!("Could not open manifest file {} - {:?}", path, error),
            Ok(file) => file,
        };
        {
            let reader = BufReader::new(&file);
            let version: VersionDetect = serde_yaml::from_reader(reader)?;
            if version.version != "1" {
                bail!("Only manifest version 1 is supported for now.");
            }
        }
        let _ = file.seek(SeekFrom::Start(0))?;
        let reader = BufReader::new(&file);
        let deserializer = Deserializer::from_reader(reader);
        let result: Result<Manifest, _> = serde_path_to_error::deserialize(deserializer);
        let manifest: Manifest = match result {
            Ok(manifest) => manifest,
            Err(error) => bail!("Failed to parse manifest at {}\n -> {}", path, error),
        };
        Ok(manifest)
    }
}

#[cfg(test)]
mod test_deserialize {

    use crate::components::{Component, Components};
    use crate::location::Location;
    use crate::lowercase::lwc;
    use crate::module::file_mod::FileModule;
    use crate::module::file_module_origin::FileModuleOrigin;
    use crate::module::gen_mod::{GeneratedMod, GenModComponent};
    use crate::module::module::Module;
    use crate::module::weidu_mod::WeiduMod;

    use super::Manifest;

    #[test]
    fn check_read_manifest() {
        let manifest_path = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), "resources/test/manifest.yml");
        let manifest = Manifest::read_path(&manifest_path).unwrap();
        assert_eq!(
            manifest,
            super::Manifest {
                version : "1".to_string(),
                global : super::Global {
                    game_language: "fr_FR".to_string(),
                    lang_preferences: Some(vec!["french".to_string()]),
                    patch_path: None,
                    local_mods: None,
                    local_files: None,
                },
                modules : vec![],
            }
        )
    }

    #[test]
    fn check_read_manifest_with_module() {
        let manifest_path = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), "resources/test/manifest_with_modules.yml");
        let manifest = Manifest::read_path(&manifest_path).unwrap();
        assert_eq!(
            manifest,
            super::Manifest {
                version : "1".to_string(),
                global : super::Global {
                    game_language: "fr_FR".to_string(),
                    lang_preferences: Some(vec!["french".to_string()]),
                    patch_path: None,
                    local_mods: Some("mods".to_string()),
                    local_files: None,
                },
                modules : vec![
                    Module::Mod {
                        weidu_mod: WeiduMod {
                            name: lwc!("aaa"),
                            components: Components::List(vec! [ Component::Simple(1) ]),
                            location: Some(Location {
                                source: crate::location::Source::Http { http: "http://example.com/my-mod".to_string(), rename: None },
                                ..Default::default()
                            }),
                            ..Default::default()
                        },
                    },
                    //Module::File {
                    //    file: FileModule {
                    //        file_mod: lwc!("bbb"),
                    //        from: FileModuleOrigin::Local { local:"files/my-file.itm".to_string(), glob: None },
                    //        to: "override".to_string(),
                    //        description: None,
                    //        post_install: None,
                    //        allow_overwrite: false,
                    //    },
                    //},
                    //Module::Generated {
                    //    gen:  GeneratedMod {
                    //        gen_mod: lwc!("ccc"),
                    //        files: vec![
                    //            FileModuleOrigin::Local { local: "my_subdir".to_string(), glob: None },
                    //        ],
                    //        description: None,
                    //        post_install: None,
                    //        component: GenModComponent { index: 0, name: None },
                    //        ignore_warnings: true,
                    //        allow_overwrite: true,
                    //    },
                    //},
                    //Module::Generated {
                    //    gen:  GeneratedMod {
                    //        gen_mod: lwc!("ddd"),
                    //        files: vec![
                    //            FileModuleOrigin::Local { local: "my_other_subdir".to_string(), glob: Some("*.itm".to_string()) },
                    //        ],
                    //        description: None,
                    //        post_install: None,
                    //        component: GenModComponent { index: 10, name: Some("Do whatever".to_string()) },
                    //        ignore_warnings: true,
                    //        allow_overwrite: true,
                    //    },
                    //},
                ],
            }
        )
    }

    #[test]
    fn serialize_manifest_with_modules() {

        let manifest = super::Manifest {
            version : "1".to_string(),
            global : super::Global {
                game_language: "fr_FR".to_string(),
                lang_preferences: Some(vec!["french".to_string()]),
                patch_path: None,
                local_mods: Some("mods".to_string()),
                local_files: None,
            },
            modules : vec![
                Module::Mod {
                    weidu_mod: WeiduMod {
                        name: lwc!("aaa"),
                        components: Components::List(vec! [ Component::Simple(1) ]),
                        location: Some(Location {
                            source: crate::location::Source::Http { http: "http://example.com/my-mod".to_string(), rename: None },
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                },
                Module::File {
                    file: FileModule {
                        file_mod: lwc!("bbb"),
                        from: FileModuleOrigin::Local { local:"files/my-file.itm".to_string(), glob: None },
                        to: "override".to_string(),
                        description: None,
                        post_install: None,
                        allow_overwrite: false,
                    },
                },
                Module::Generated {
                    gen:  GeneratedMod {
                        gen_mod: lwc!("ccc"),
                        files: vec![
                            FileModuleOrigin::Local { local: "my_subdir".to_string(), glob: None },
                        ],
                        description: None,
                        post_install: None,
                        component: GenModComponent { index: 0, name: None },
                        ignore_warnings: true,
                        allow_overwrite: true,
                    },
                },
                Module::Generated {
                    gen:  GeneratedMod {
                        gen_mod: lwc!("ddd"),
                        files: vec![
                            FileModuleOrigin::Local { local: "my_other_subdir".to_string(), glob: Some("*.itm".to_string()) },
                        ],
                        description: None,
                        post_install: None,
                        component: GenModComponent { index: 10, name: Some("Do whatever".to_string()) },
                        ignore_warnings: true,
                        allow_overwrite: true,
                    },
                },
            ],
        };

        println!("{}", serde_yaml::to_string(&manifest).unwrap());
    }
}
