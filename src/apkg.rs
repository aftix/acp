use crate::deck;
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    io::Write,
    path::{Path, PathBuf},
};
use tempfile;
use zip;

// Owns the temporary extracted Apkg and the collection
#[derive(Debug)]
pub struct Apkg {
    dir: tempfile::TempDir,
    db_path: PathBuf,
    media_path: PathBuf,
    collection: deck::Collection,
    media: Vec<Media>,
}

// Media files in the apkg
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Media {
    path: PathBuf,
    name: String,
}

fn load_media(path: &Path) -> io::Result<Vec<Media>> {
    let mut vec = Vec::new();

    let contents = fs::read_to_string(path)?;
    let json = json::parse(&contents).expect("Media JSON is not JSON");
    if !json.is_object() {
        return Ok(vec);
    }

    let dir = path.parent().unwrap();
    for (condensed_name, value) in json.entries() {
        if let Some(val) = value.as_str() {
            let name = String::from(val);
            let mediapath = dir.join(condensed_name);
            vec.push(Media {
                path: mediapath,
                name,
            });
        }
    }

    Ok(vec)
}

// Path is path to "media", v is the entries in the JSON
fn save_media(path: &Path, v: Vec<Media>) -> io::Result<()> {
    fs::remove_file(path)?;
    let mut json = object! {};

    for media in v.into_iter() {
        let name = media.path.file_name().unwrap();
        json.insert(name.to_str().unwrap(), media.name).unwrap();
    }

    let json_text = json::stringify(json);

    let mut file = fs::File::create(path)?;
    file.write_all(json_text.as_bytes())
}

impl Apkg {
    // Extract an apkg into a temporary directory which is owned by the resulting struct
    pub fn new(path: &Path) -> io::Result<Self> {
        // Open the zip archive
        let file = fs::File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        // Make a temporary directory that will be owned by the resultant Apkg
        let dir = tempfile::tempdir()?;

        // Extract the contents of the zip file to the temporary directory
        for i in 0..archive.len() {
            // Get the path of the file
            let mut file = archive.by_index(i)?;
            let outpath = match file.enclosed_name() {
                Some(path) => path.to_owned(),
                None => continue,
            };

            let outpath = dir.path().join(outpath);

            if (&*file.name()).ends_with("/") {
                // File is a directory, create it in tempdir
                fs::create_dir_all(&outpath)?;
            } else {
                // File is not a directory, extract it
                if let Some(p) = outpath.parent() {
                    // Create directory if needed
                    if !p.exists() {
                        fs::create_dir_all(&p)?;
                    }
                }
                let mut outfile = fs::File::create(&outpath)?;
                io::copy(&mut file, &mut outfile)?;
            }

            // Set permissions on unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = file.unix_mode() {
                    fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))?;
                }
            }
        }

        let db_path = dir.path().join("collection.anki2");
        let media_path = dir.path().join("media");
        let collection = deck::Collection::new(db_path.as_path());
        if let Err(err) = collection {
            return Err(io::Error::new(io::ErrorKind::Other, err));
        }
        let collection = collection.unwrap();

        let media = load_media(media_path.as_path())?;

        let apkg = Apkg {
            dir,
            db_path,
            media_path,
            collection,
            media,
        };

        Ok(apkg)
    }

    pub fn save(self, path: &Path) -> io::Result<()> {
        // Write to temporary directory
        save_media(self.media_path.as_path(), self.media)?;
        if let Err(err) = self.collection.save(self.db_path.as_path()) {
            return Err(io::Error::new(io::ErrorKind::Other, err));
        }

        // Zip the archive
        let file = fs::File::create(path)?;
        let mut zip = zip::ZipWriter::new(file);
        let options =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);

        let dir_path = self.dir.path();
        let paths = fs::read_dir(dir_path)?;

        for path in paths {
            if let Err(err) = path {
                return Err(io::Error::new(io::ErrorKind::Other, err));
            }
            let path = path.unwrap();
            if let Err(err) =
                zip.start_file(path.path().file_name().unwrap().to_str().unwrap(), options)
            {
                return Err(io::Error::new(io::ErrorKind::Other, err));
            }

            let contents = fs::read(path.path())?;
            if let Err(err) = zip.write(&contents[..]) {
                return Err(io::Error::new(io::ErrorKind::Other, err));
            }
        }

        // Finish
        if let Err(err) = zip.finish() {
            return Err(io::Error::new(io::ErrorKind::Other, err));
        }

        Ok(())
    }
}
