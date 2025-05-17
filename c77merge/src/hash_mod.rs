use std::{
    fs::{File, read_dir},
    io,
    path::{Path, PathBuf},
};

use md5::{Digest, Md5};
use miette::miette;
use parking_lot::Mutex;
use rayon::prelude::*;
use tracing::{Span, debug, info, instrument};

use crate::error::{C77Error, HashErrors};

#[instrument(level = "trace")]
fn hash_mod(path: &PathBuf) -> Result<String, C77Error> {
    info!("Hashing {path:?}");
    let mut file = File::open(path)?;
    let mut hasher = Md5::new();
    io::copy(&mut file, &mut hasher)?;
    Ok(base16ct::lower::encode_string(&hasher.finalize()))
}

#[instrument(level = "trace")]
pub fn hash_all_in_dir(path: &Path) -> Result<(Vec<String>, HashErrors), C77Error> {
    info!("Hashing all mods...");

    let dld = read_dir(path);

    let errors = Vec::new();
    let err_mut = Mutex::new(errors);

    let us = &Span::current();

    // save errors in the errors vec, we'll handle them later
    let mod_hashes: Vec<String> = dld
        // .into_diagnostic()
        .map_err(|e| miette!("Could not read directory {}: {e}", &path.display()))?
        .collect::<Vec<_>>()
        // uses rayon to hash in parallel
        .into_par_iter()
        .filter_map(|d| {
            d.map(|f| f.path())
                .map(|p| {
                    us.in_scope(|| {
                        let hash = hash_mod(&p);
                        match hash {
                            Ok(ref h) => {
                                debug!("hash: {h}");
                            }
                            Err(_) => {
                                debug!("hash: {hash:?}");
                            }
                        };
                        hash
                    })
                })
                .ok()
        })
        .into_par_iter()
        .filter_map(|d| {
            d.map_err(|e| {
                let mut errs = err_mut.lock();
                errs.push(e);
            })
            .ok()
        })
        .collect();

    let errors = err_mut.into_inner().into();

    Ok((mod_hashes, errors))
}
