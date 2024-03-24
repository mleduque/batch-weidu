
use anyhow::{Result, bail};
use chrono::Utc;
use handlebars::Handlebars;
use serde_json::json;

use std::io::Write;

use crate::canon_path::CanonPath;
use crate::module::gen_mod::GeneratedMod;

const TP2_TEMPLATE: &str ="
/*
 * TP2 generated by modda
 * {{date}}
*/
BACKUP ~weidu_external//backup/{{mod_name}}~
AUTHOR ~generated by modda~

BEGIN ~{{component_name}}~
DESIGNATED ~{{index}}~

COPY ~{{mod_name}}/data~ ~override~

";

pub fn generate_tp2(gen: &GeneratedMod) -> Result<String> {
    let registry = Handlebars::new();
    let comp_name = match &gen.component.name {
        None => gen.gen_mod.to_string(),
        Some(s) if s.is_empty() => gen.gen_mod.to_string(),
        Some(name) => name.to_owned(),
    };
    let result = registry.render_template(
        TP2_TEMPLATE,
        &json!({
            "date": Utc::now().to_string(),
            "mod_name": &gen.gen_mod,
            "component_name": comp_name,
            "index": gen.component.index,
        })
    )?;
    Ok(result)
}

pub fn create_tp2(gen: &GeneratedMod, target: &CanonPath) -> Result<()> {
    let content = match generate_tp2(gen) {
        Err(err) => bail!("Could not generate tp2 file from template\n  {}", err),
        Ok(content) => content,
    };
    let tp2_path = target.join(format!("{}.tp2", gen.gen_mod))?;
    let file = std::fs::OpenOptions::new()
                                            .write(true)
                                            .create_new(true)
                                            .open(tp2_path);
    let mut file = match file {
        Err(err) => bail!("Could not create generated tp2 file {}\n  {}", gen.gen_mod, err),
        Ok(file) => file,
    };
    if let Err(err) = write!(file, "{}", content) {
        bail!("Could not write content to generated tp2 file {}\n  {}", gen.gen_mod, err);
    }
    Ok(())
}