use serde::{Deserialize, Serialize};
use std::{fs, path::{Path, PathBuf}, process::Command};
use tauri::{Manager, Emitter};
use sha2::{Digest, Sha256};
use rusqlite::{Connection, params, OptionalExtension};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all="camelCase")]
struct Game { id: String, name: String, path: String, sha256: String, #[serde(default)] cover_path: Option<String> }
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all="camelCase")]
struct ModFile { id: String, name: String, path: String }
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all="camelCase")]
struct Profile { id: String, name: String, game_id: String, engine_path: String, mod_ids: Vec<String>, #[serde(default)] map_ids: Vec<String>, #[serde(default)] cover_path: Option<String>, extra_args: Vec<String>, #[serde(default)] group: Option<String>, #[serde(default)] play_seconds: u64, #[serde(default)] launches: u64 }
#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all="camelCase")]
struct Library { games: Vec<Game>, #[serde(default)] engines: Vec<EngineInfo>, mods: Vec<ModFile>, #[serde(default)] maps: Vec<ModFile>, profiles: Vec<Profile> }
fn data_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
 let path = app.path().app_data_dir().map_err(|e| e.to_string())?;
 fs::create_dir_all(&path).map_err(|e| e.to_string())?;
 Ok(path)
}
// SQLite is the canonical storage. JSON is only an optional user-created backup format.
fn database(app:&tauri::AppHandle)->Result<Connection,String>{
 let dir=data_path(app)?;
 let conn=Connection::open(dir.join("hellforge.sqlite3")).map_err(|e|e.to_string())?;
 conn.busy_timeout(std::time::Duration::from_secs(5)).map_err(|e|e.to_string())?;
 conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; CREATE TABLE IF NOT EXISTS library_snapshot (id INTEGER PRIMARY KEY CHECK(id=1), payload TEXT NOT NULL, updated_at TEXT DEFAULT CURRENT_TIMESTAMP); CREATE TABLE IF NOT EXISTS play_history (id INTEGER PRIMARY KEY AUTOINCREMENT, profile_id TEXT NOT NULL, started_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, seconds INTEGER NOT NULL, exit_code INTEGER); CREATE INDEX IF NOT EXISTS idx_play_history_profile ON play_history(profile_id); CREATE TABLE IF NOT EXISTS completed_profiles (profile_id TEXT PRIMARY KEY, completed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);").map_err(|e|e.to_string())?;
 Ok(conn)
}
fn load(app: &tauri::AppHandle) -> Result<Library,String> {
 let conn=database(app)?;
 let payload:Option<String>=conn.query_row("SELECT payload FROM library_snapshot WHERE id=1",[],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
 if let Some(payload)=payload{return serde_json::from_str(&payload).map_err(|e|e.to_string())}
 Ok(Library::default())
}
fn save(app: &tauri::AppHandle, data: &Library) -> Result<(),String> {
 let mut conn=database(app)?;
 let json=serde_json::to_string(data).map_err(|e|e.to_string())?;
 let tx=conn.transaction().map_err(|e|e.to_string())?;
 tx.execute("INSERT INTO library_snapshot(id,payload) VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload,updated_at=CURRENT_TIMESTAMP",params![json]).map_err(|e|e.to_string())?;
 tx.commit().map_err(|e|e.to_string())?;
 Ok(())
}
#[tauri::command]
fn backup_library(app:tauri::AppHandle,path:String)->Result<String,String>{
 let lib=load(&app)?;
 let dest=PathBuf::from(path);
 let tmp=dest.with_extension("hellforge.tmp");
 fs::write(&tmp,serde_json::to_vec_pretty(&lib).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
 if dest.exists(){fs::remove_file(&dest).map_err(|e|e.to_string())?}
 fs::rename(&tmp,&dest).map_err(|e|e.to_string())?;
 Ok(dest.to_string_lossy().into_owned())
}
#[tauri::command]
fn restore_library(app:tauri::AppHandle,path:String)->Result<Library,String>{
 let bytes=fs::read(path).map_err(|e|e.to_string())?;
 if bytes.len()>32*1024*1024{return Err("Library backup exceeds 32 MB".into())}
 let incoming:Library=serde_json::from_slice(&bytes).map_err(|e|format!("Invalid backup: {e}"))?;
 // Only restore metadata; missing referenced files remain visible for repair.
 let existing=load(&app)?;
 let archive=data_path(&app)?.join("library-before-restore.json");
 fs::write(archive,serde_json::to_vec_pretty(&existing).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
 save(&app,&incoming)?;Ok(incoming)
}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
struct PlaySession{profile_id:String,started_at:String,seconds:u64,exit_code:Option<i32>}
#[tauri::command]
fn get_play_history(app:tauri::AppHandle,profile_id:String)->Result<Vec<PlaySession>,String>{
 let conn=database(&app)?;
 let mut stmt=conn.prepare("SELECT profile_id,started_at,seconds,exit_code FROM play_history WHERE profile_id=?1 ORDER BY id DESC LIMIT 100").map_err(|e|e.to_string())?;
 let rows=stmt.query_map([profile_id],|r|Ok(PlaySession{profile_id:r.get(0)?,started_at:r.get(1)?,seconds:r.get(2)?,exit_code:r.get(3)?})).map_err(|e|e.to_string())?;
 rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())
}
fn file_id(p:&Path)->Result<String,String>{
 let bytes=fs::read(p).map_err(|e|e.to_string())?;
 Ok(hex::encode(Sha256::digest(bytes)))
}
#[tauri::command]
fn get_library(app:tauri::AppHandle)->Result<Library,String>{load(&app)}
fn managed_copy(app:&tauri::AppHandle, source:&Path, category:&str, id:&str)->Result<String,String>{
 let name=source.file_name().ok_or("Invalid filename")?.to_string_lossy();
 let folder=data_path(app)?.join(category);fs::create_dir_all(&folder).map_err(|e|e.to_string())?;
 let destination=folder.join(format!("{}-{}",&id[..16],name));
 if !destination.exists(){fs::copy(source,&destination).map_err(|e|e.to_string())?;}
 Ok(destination.to_string_lossy().into_owned())
}
#[tauri::command]
fn import_game(app:tauri::AppHandle,path:String)->Result<Library,String>{
 let p=PathBuf::from(&path);
 if !p.is_file() || !p.extension().is_some_and(|e|e.eq_ignore_ascii_case("wad")) {return Err("Виберіть існуючий IWAD .wad".into());}
 let id=file_id(&p)?;
 let mut l=load(&app)?;
 if !l.games.iter().any(|g|g.id==id){
  let details=inspect_wad_file(&p)?;if !details.is_iwad{return Err("This file is a PWAD; import it as a mod or map".into())}
  let stem=p.file_stem().unwrap_or_default().to_string_lossy().to_uppercase();
  let name=match stem.as_str(){"DOOM"=>"DOOM","DOOM2"=>"DOOM II","DOOMU"=>"Ultimate DOOM","TNT"=>"Final DOOM: TNT","PLUTONIA"=>"Final DOOM: Plutonia","HERETIC"=>"Heretic","HEXEN"=>"Hexen","FREEDOOM1"=>"Freedoom Phase 1","FREEDOOM2"=>"Freedoom Phase 2",_=>stem.as_str()}.to_string();
  let cover_path=steamgrid_cover(&app,&name,&stem).ok().flatten();
  let stored=managed_copy(&app,&p,"games",&id)?;
  l.games.push(Game{id:id.clone(),name,path:stored,sha256:id,cover_path});
 }
 save(&app,&l)?;Ok(l)
}
// Fetch official Steam library art on IWAD import; cache it for offline use.
// Covers remain optional and may be replaced with user-selected images.
fn fetch_official_cover(app:&tauri::AppHandle,stem:&str)->Result<Option<String>,String>{
 let steam_id=match stem {"DOOM"|"DOOMU"=>"2280","DOOM2"=>"2300","TNT"|"PLUTONIA"=>"2290","HERETIC"=>"2390","HEXEN"=>"2360",_=>return Ok(None)};
 let dir=data_path(app)?.join("game-covers");fs::create_dir_all(&dir).map_err(|e|e.to_string())?;
 let dest=dir.join(format!("{}.jpg",stem.to_ascii_lowercase()));
 if dest.is_file(){return Ok(Some(dest.to_string_lossy().into_owned()))}
 let url=format!("https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/{steam_id}/library_600x900.jpg");
 let client=reqwest::blocking::Client::builder().timeout(std::time::Duration::from_secs(7)).build().map_err(|e|e.to_string())?;
 let response=client.get(url).send().map_err(|e|e.to_string())?.error_for_status().map_err(|e|e.to_string())?;
 let bytes=response.bytes().map_err(|e|e.to_string())?;
 if bytes.len()<2000||bytes.len()>8_000_000||!bytes.starts_with(&[0xff,0xd8,0xff]){return Ok(None)}
 fs::write(&dest,&bytes).map_err(|e|e.to_string())?;
 Ok(Some(dest.to_string_lossy().into_owned()))
}
#[tauri::command]
fn save_steamgrid_key(app:tauri::AppHandle,key:String)->Result<(),String>{
 let path=data_path(&app)?.join("steamgriddb.key");
 if key.trim().is_empty(){if path.exists(){fs::remove_file(path).map_err(|e|e.to_string())?;}return Ok(())}
 if key.len()>256{return Err("API key too long".into())}
 fs::write(path,key.trim()).map_err(|e|e.to_string())
}
fn steamgrid_cover(app:&tauri::AppHandle,name:&str,stem:&str)->Result<Option<String>,String>{
 let key_path=data_path(app)?.join("steamgriddb.key");if !key_path.exists(){return Ok(None)}
 let key=fs::read_to_string(key_path).map_err(|e|e.to_string())?;
 let client=reqwest::blocking::Client::builder().timeout(std::time::Duration::from_secs(15)).build().map_err(|e|e.to_string())?;
 let search:serde_json::Value=client.get(format!("https://www.steamgriddb.com/api/v2/search/autocomplete/{}",name))
  .header("Authorization",format!("Bearer {}",key)).send().map_err(|e|e.to_string())?.error_for_status().map_err(|e|e.to_string())?.json().map_err(|e|e.to_string())?;
 let game_id=match search["data"].as_array().and_then(|a|a.first()).and_then(|v|v["id"].as_u64()){Some(id)=>id,None=>return Ok(None)};
 let result:serde_json::Value=client.get(format!("https://www.steamgriddb.com/api/v2/grids/game/{}",game_id))
  .query(&[("dimensions","600x900")]).header("Authorization",format!("Bearer {}",key))
  .send().map_err(|e|e.to_string())?.error_for_status().map_err(|e|e.to_string())?.json().map_err(|e|e.to_string())?;
 let url=match result["data"].as_array().and_then(|a|a.first()).and_then(|v|v["url"].as_str()){Some(u)=>u,None=>return Ok(None)};
 let parsed=reqwest::Url::parse(url).map_err(|e|e.to_string())?;
 if parsed.scheme()!="https" || !parsed.host_str().is_some_and(|h|h=="steamgriddb.com"||h.ends_with(".steamgriddb.com")){return Err("Unexpected cover host".into())}
 let bytes=client.get(url).send().map_err(|e|e.to_string())?.error_for_status().map_err(|e|e.to_string())?.bytes().map_err(|e|e.to_string())?;
 if bytes.len()>12_000_000{return Err("Cover too large".into())}
 let ext=if bytes.starts_with(&[0x89,0x50,0x4e,0x47]){"png"}else if bytes.starts_with(&[0xff,0xd8,0xff]){"jpg"}else if bytes.starts_with(b"RIFF"){"webp"}else{return Err("Unsupported image".into())};
 let dir=data_path(app)?.join("game-covers");fs::create_dir_all(&dir).map_err(|e|e.to_string())?;
 let dest=dir.join(format!("sgdb-{}.{}",stem.to_ascii_lowercase(),ext));fs::write(&dest,bytes).map_err(|e|e.to_string())?;
 Ok(Some(dest.to_string_lossy().into_owned()))
}
#[tauri::command]
async fn fetch_profile_cover(app:tauri::AppHandle,profile_id:String)->Result<Library,String>{
 tauri::async_runtime::spawn_blocking(move||{
  let mut lib=load(&app)?;
  let profile=lib.profiles.iter().find(|p|p.id==profile_id).ok_or("Profile not found")?;
  let name=profile.name.clone();
  let stem=format!("profile-{}",profile.id.chars().filter(|c|c.is_ascii_alphanumeric()).collect::<String>());
  let cover=steamgrid_cover(&app,&name,&stem)?.ok_or("No 600x900 SteamGridDB cover found for this name")?;
  let p=lib.profiles.iter_mut().find(|p|p.id==profile_id).ok_or("Profile not found")?;
  p.cover_path=Some(cover);
  save(&app,&lib)?;Ok(lib)
 }).await.map_err(|e|e.to_string())?
}
#[tauri::command]
async fn refresh_steamgrid_covers(app:tauri::AppHandle)->Result<Library,String>{
 tauri::async_runtime::spawn_blocking(move||{
  let mut lib=load(&app)?;
  for game in &mut lib.games {
   let stem=Path::new(&game.path).file_name().unwrap_or_default().to_string_lossy().to_ascii_uppercase();
   let name=game.name.clone();
   if let Ok(Some(cover))=steamgrid_cover(&app,&name,&stem){game.cover_path=Some(cover)}
  }
  save(&app,&lib)?;Ok(lib)
 }).await.map_err(|e|e.to_string())?
}
#[tauri::command]
async fn refresh_game_covers(app:tauri::AppHandle)->Result<Library,String>{
 tauri::async_runtime::spawn_blocking(move||{
  let mut lib=load(&app)?;
  for game in &mut lib.games {
   if game.cover_path.as_ref().is_some_and(|p|Path::new(p).is_file()){continue}
   let stem=Path::new(&game.path).file_stem().unwrap_or_default().to_string_lossy().to_uppercase();
   game.cover_path=fetch_official_cover(&app,&stem).ok().flatten();
  }
  save(&app,&lib)?;Ok(lib)
 }).await.map_err(|e|e.to_string())?
}
#[tauri::command]
fn save_game_cover(app:tauri::AppHandle,game_id:String,path:String)->Result<Library,String>{
 let ext=Path::new(&path).extension().and_then(|s|s.to_str()).unwrap_or("").to_lowercase();
 if !["png","jpg","jpeg","webp"].contains(&ext.as_str()){return Err("Unsupported cover format".into())}
 if fs::metadata(&path).map_err(|e|e.to_string())?.len()>12_000_000{return Err("Cover exceeds 12MB".into())}
 let mut lib=load(&app)?;
 let game=lib.games.iter_mut().find(|g|g.id==game_id).ok_or("Game not found")?;
 let dir=data_path(&app)?.join("game-covers");fs::create_dir_all(&dir).map_err(|e|e.to_string())?;
 let dest=dir.join(format!("custom-{}.{}",game_id,ext));fs::copy(path,&dest).map_err(|e|e.to_string())?;
 game.cover_path=Some(dest.to_string_lossy().into_owned());save(&app,&lib)?;Ok(lib)
}
#[tauri::command]
fn import_mod(app:tauri::AppHandle,path:String,kind:Option<String>)->Result<Library,String>{
 let p=PathBuf::from(&path);
 if !p.is_file() || !p.extension().is_some_and(|e|["wad","pk3","pk7","zip"].iter().any(|x|e.eq_ignore_ascii_case(x))){return Err("Підтримуються WAD, PK3, PK7, ZIP".into());}
 let id=file_id(&p)?;let mut l=load(&app)?;
 let target=if kind.as_deref()==Some("map") {&mut l.maps} else {&mut l.mods};
 if !target.iter().any(|m|m.id==id){let stored=managed_copy(&app,&p,if kind.as_deref()==Some("map"){"maps"}else{"mods"},&id)?;target.push(ModFile{id,name:p.file_name().unwrap_or_default().to_string_lossy().into_owned(),path:stored});}
 save(&app,&l)?;Ok(l)
}
#[tauri::command]
fn rename_library_item(app:tauri::AppHandle,kind:String,id:String,name:String)->Result<Library,String>{
 let clean=name.trim();
 if clean.is_empty() || clean.chars().count()>120 || clean.chars().any(|c|c.is_control()) {return Err("Invalid display name (1–120 characters)".into())}
 let mut lib=load(&app)?;
 match kind.as_str(){
  "mod"=>lib.mods.iter_mut().find(|x|x.id==id).ok_or("Mod not found")?.name=clean.into(),
  "map"=>lib.maps.iter_mut().find(|x|x.id==id).ok_or("Map not found")?.name=clean.into(),
  _=>return Err("Only mods and maps can be renamed".into())
 }
 save(&app,&lib)?;Ok(lib)
}
#[tauri::command]
fn reveal_library_folder(app:tauri::AppHandle,kind:String)->Result<(),String>{
 let folder=match kind.as_str(){"game"=>"games","mod"=>"mods","map"=>"maps","cover"=>"covers",_=>return Err("Unknown library folder".into())};
 let path=data_path(&app)?.join(folder);
 fs::create_dir_all(&path).map_err(|e|e.to_string())?;
 #[cfg(target_os="windows")]
 {Command::new("explorer.exe").arg(&path).spawn().map_err(|e|e.to_string())?;}
 #[cfg(target_os="linux")]
 {Command::new("xdg-open").arg(&path).spawn().map_err(|e|e.to_string())?;}
 #[cfg(target_os="macos")]
 {Command::new("open").arg(&path).spawn().map_err(|e|e.to_string())?;}
 Ok(())
}
#[tauri::command]
fn delete_library_item(app:tauri::AppHandle,kind:String,id:String)->Result<Library,String>{
 let mut lib=load(&app)?;
 let file=match kind.as_str(){
  "game"=>lib.games.iter().find(|x|x.id==id).map(|x|x.path.clone()),
  "mod"=>lib.mods.iter().find(|x|x.id==id).map(|x|x.path.clone()),
  "map"=>lib.maps.iter().find(|x|x.id==id).map(|x|x.path.clone()),
  _=>return Err("Unknown library category".into())
 }.ok_or("Item not found")?;
 // Never delete arbitrary external paths: only files within our managed category.
 let category=match kind.as_str(){"game"=>"games","mod"=>"mods",_=>"maps"};
 let managed=data_path(&app)?.join(category).canonicalize().map_err(|e|e.to_string())?;
 let actual=PathBuf::from(&file).canonicalize().map_err(|e|e.to_string())?;
 if !actual.starts_with(&managed){return Err("File is outside managed library".into())}
 if kind=="game" && lib.profiles.iter().any(|p|p.game_id==id){return Err("Remove profiles using this game first".into())}
 if kind=="game"{lib.games.retain(|x|x.id!=id)}
 else if kind=="mod"{lib.mods.retain(|x|x.id!=id);for p in &mut lib.profiles{p.mod_ids.retain(|x|x!=&id)}}
 else{lib.maps.retain(|x|x.id!=id);for p in &mut lib.profiles{p.map_ids.retain(|x|x!=&id)}}
 save(&app,&lib)?;
 fs::remove_file(actual).map_err(|e|format!("Library updated, but could not delete file: {e}"))?;
 Ok(lib)
}
#[tauri::command]
fn save_profile(app:tauri::AppHandle,profile:Profile)->Result<Library,String>{
 let mut l=load(&app)?;
 if profile.name.trim().is_empty(){return Err("Назва профілю обов'язкова".into());}
 if !l.games.iter().any(|g|g.id==profile.game_id){return Err("IWAD не знайдено".into());}
 if let Some(old)=l.profiles.iter_mut().find(|p|p.id==profile.id){*old=profile;}else{l.profiles.push(profile);}
 save(&app,&l)?;Ok(l)
}
#[tauri::command]
fn delete_profile(app:tauri::AppHandle,id:String)->Result<Library,String>{let mut l=load(&app)?;l.profiles.retain(|p|p.id!=id);save(&app,&l)?;Ok(l)}
#[tauri::command]
fn launch_profile(app:tauri::AppHandle,id:String)->Result<String,String>{
 let l=load(&app)?;
 let p=l.profiles.iter().find(|p|p.id==id).ok_or("Профіль не знайдено")?;
 let game=l.games.iter().find(|g|g.id==p.game_id).ok_or("IWAD не знайдено")?;
 let engine=PathBuf::from(&p.engine_path);
 if !engine.is_file() || !engine.extension().is_some_and(|x|x.eq_ignore_ascii_case("exe")){return Err("Вкажіть шлях до виконуваного файлу рушія (.exe)".into());}
 if !Path::new(&game.path).is_file(){return Err("Файл IWAD більше не існує".into());}
 let mut cmd=Command::new(&engine);
 cmd.arg("-iwad").arg(&game.path);
 let selected:Vec<&ModFile>=p.mod_ids.iter().chain(p.map_ids.iter()).map(|id|l.mods.iter().chain(l.maps.iter()).find(|m|&m.id==id).ok_or(format!("Файл {id} не знайдено"))).collect::<Result<_,_>>()?;
 if !selected.is_empty(){cmd.arg("-file");for m in selected{if !Path::new(&m.path).is_file(){return Err(format!("Відсутній мод: {}",m.name));}cmd.arg(&m.path);}}
 // Arguments are passed as separate values: no shell interpolation.
 for arg in &p.extra_args {cmd.arg(arg);}
 if let Some(parent)=engine.parent(){cmd.current_dir(parent);}
 let mut child=cmd.spawn().map_err(|e|format!("Не вдалося запустити рушій: {e}"))?;
 let pid=child.id();
 let _=app.emit("hellforge-launch",serde_json::json!({"profileId":id,"state":"started","pid":pid}));
 let handle=app.clone(); let profile_id=id.clone();
 std::thread::spawn(move || {
  let started=std::time::Instant::now();
  let result=child.wait();
  if let Ok(mut lib)=load(&handle){if let Some(profile)=lib.profiles.iter_mut().find(|p|p.id==profile_id){profile.play_seconds=profile.play_seconds.saturating_add(started.elapsed().as_secs());profile.launches=profile.launches.saturating_add(1);let _=save(&handle,&lib);}}
  if let Ok(conn)=database(&handle){let _=conn.execute("INSERT INTO play_history(profile_id,seconds,exit_code) VALUES(?1,?2,?3)",params![profile_id,started.elapsed().as_secs(),result.as_ref().ok().and_then(|status|status.code())]);}
  let (state,detail)=match result {Ok(status)=>("exited",format!("Гра завершилася (код: {:?})",status.code())),Err(e)=>("error",format!("Помилка очікування процесу: {e}"))};
  let _=handle.emit("hellforge-launch",serde_json::json!({"profileId":profile_id,"state":state,"detail":detail}));
 });
 Ok(format!("Гру запущено (PID {})",pid))
}
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all="camelCase")]
struct EngineInfo { name:String, version:String, path:String }
fn http() -> Result<reqwest::blocking::Client,String> {
 reqwest::blocking::Client::builder().user_agent("Hellforge/0.3 (desktop launcher)").timeout(std::time::Duration::from_secs(90)).build().map_err(|e|e.to_string())
}
fn engines_dir(app:&tauri::AppHandle)->Result<PathBuf,String>{let p=data_path(app)?.join("engines");fs::create_dir_all(&p).map_err(|e|e.to_string())?;Ok(p)}
#[tauri::command]
fn list_engines(app:tauri::AppHandle)->Result<Vec<EngineInfo>,String>{
 Ok(load(&app)?.engines.into_iter().filter(|e|Path::new(&e.path).is_file()).collect())
}
#[tauri::command]
fn register_engine(app:tauri::AppHandle,path:String,name:String)->Result<Vec<EngineInfo>,String>{
 let p=Path::new(&path);
 if !p.is_file()||!p.extension().is_some_and(|e|e.eq_ignore_ascii_case("exe")){return Err("Select a valid .exe".into())}
 let mut l=load(&app)?;
 if !l.engines.iter().any(|e|e.path.eq_ignore_ascii_case(&path)){
  l.engines.push(EngineInfo{name:if name.trim().is_empty(){p.file_stem().unwrap_or_default().to_string_lossy().into_owned()}else{name},version:"Local".into(),path});
 }
 save(&app,&l)?;list_engines(app)
}
#[tauri::command]
fn unregister_engine(app:tauri::AppHandle,path:String)->Result<Vec<EngineInfo>,String>{
 let mut l=load(&app)?;l.engines.retain(|e|!e.path.eq_ignore_ascii_case(&path));save(&app,&l)?;list_engines(app)
}
#[tauri::command]
async fn install_engine(app:tauri::AppHandle,kind:String)->Result<EngineInfo,String>{
 tauri::async_runtime::spawn_blocking(move||install_engine_blocking(app,&kind)).await.map_err(|e|e.to_string())?
}
fn install_engine_blocking(app:tauri::AppHandle,kind:&str)->Result<EngineInfo,String>{
 let (name,repo,exe_names)=match kind {
  "gzdoom"=>("GZDoom","ZDoom/gzdoom",vec!["gzdoom.exe"]),
  "lzdoom"=>("LZDoom","drfrag666/lzdoom",vec!["lzdoom.exe"]),
  "dsda"=>("DSDA-Doom","dsda-org/dsda-doom",vec!["dsda-doom.exe"]),
  _=>return Err("Unsupported automatic engine; add its EXE manually".into())
 };
 let client=http()?;
 let api=format!("https://api.github.com/repos/{repo}/releases/latest");
 let release:serde_json::Value=client.get(&api).send().map_err(|e|e.to_string())?.error_for_status().map_err(|e|e.to_string())?.json().map_err(|e|e.to_string())?;
 let version=release["tag_name"].as_str().ok_or("Release version missing")?.to_string();
 if !version.chars().all(|c|c.is_ascii_alphanumeric()||".-_".contains(c)){return Err("Invalid release version".into())}
 let assets=release["assets"].as_array().ok_or("No release assets")?;
 let asset=assets.iter().filter_map(|a|a["name"].as_str().map(|n|(a,n.to_ascii_lowercase()))).find(|(_,n)|{
  n.ends_with(".zip") && !n.contains("source") && !n.contains("arm") && !n.contains("linux") && !n.contains("mac") && !n.contains("osx") && !n.contains("32bit") && !n.contains("win32") && (n.contains("win")||n.contains("windows")||n.contains("x64")||n.contains("64bit")||n.contains("64-bit"))
 }).ok_or("No compatible Windows ZIP in latest release")?;
 let url=asset.0["browser_download_url"].as_str().ok_or("Release download URL missing")?;
 if !url.starts_with(&format!("https://github.com/{repo}/releases/download/")){return Err("Untrusted release URL".into())}
 let bytes=client.get(url).send().map_err(|e|e.to_string())?.error_for_status().map_err(|e|e.to_string())?.bytes().map_err(|e|e.to_string())?;
 if bytes.len()>350_000_000{return Err("Download exceeds 350 MB".into())}
 let base=engines_dir(&app)?;
 let target=base.join(format!("{}-{}",kind,version));
 let staging=base.join(format!("{}-{}.partial",kind,version));
 if staging.exists(){fs::remove_dir_all(&staging).map_err(|e|e.to_string())?}
 fs::create_dir_all(&staging).map_err(|e|e.to_string())?;
 let unpack=(||->Result<PathBuf,String>{
  let mut archive=zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e|e.to_string())?;
  let mut total=0u64;
  for i in 0..archive.len(){
   let mut file=archive.by_index(i).map_err(|e|e.to_string())?;
   let safe=file.enclosed_name().ok_or("Unsafe archive entry")?;
   total=total.saturating_add(file.size());
   if file.size()>500_000_000||total>900_000_000{return Err("Archive too large".into())}
   let output=staging.join(safe);
   if file.is_dir(){fs::create_dir_all(&output).map_err(|e|e.to_string())?}
   else{if let Some(parent)=output.parent(){fs::create_dir_all(parent).map_err(|e|e.to_string())?}
    let mut dest=fs::File::create(output).map_err(|e|e.to_string())?;
    std::io::copy(&mut file,&mut dest).map_err(|e|e.to_string())?;}
  }
  let mut pending=vec![staging.clone()];
  while let Some(dir)=pending.pop(){
   for entry in fs::read_dir(dir).map_err(|e|e.to_string())?{
    let p=entry.map_err(|e|e.to_string())?.path();
    if p.is_dir(){pending.push(p)}
    else if p.file_name().and_then(|n|n.to_str()).is_some_and(|n|exe_names.iter().any(|x|n.eq_ignore_ascii_case(x))){
     return Ok(p.strip_prefix(&staging).map_err(|e|e.to_string())?.to_path_buf())
    }
   }
  }
  Err(format!("{name} executable not found in archive"))
 })();
 if unpack.is_err(){let _=fs::remove_dir_all(&staging);}
 let relative=unpack?;
 if target.exists(){fs::remove_dir_all(&target).map_err(|e|e.to_string())?}
 fs::rename(&staging,&target).map_err(|e|e.to_string())?;
 let installed=EngineInfo{name:name.into(),version,path:target.join(relative).to_string_lossy().into_owned()};
 let mut lib=load(&app)?;
 lib.engines.retain(|e|!e.path.eq_ignore_ascii_case(&installed.path));
 lib.engines.push(installed.clone());save(&app,&lib)?;
 Ok(installed)
}
#[tauri::command]
fn save_cover(app:tauri::AppHandle,profile_id:String,path:String)->Result<String,String>{
 let source=PathBuf::from(&path);
 let ext=source.extension().and_then(|x|x.to_str()).unwrap_or("").to_ascii_lowercase();
 if !["png","jpg","jpeg","webp"].contains(&ext.as_str()){return Err("Підтримуються PNG, JPG, WEBP".into())}
 let meta=fs::metadata(&source).map_err(|e|e.to_string())?;
 if meta.len()>12_000_000{return Err("Обкладинка завелика (максимум 12 MB)".into())}
 let mut l=load(&app)?;
 let profile=l.profiles.iter_mut().find(|p|p.id==profile_id).ok_or("Спочатку збережіть збірку")?;
 let dir=data_path(&app)?.join("covers");fs::create_dir_all(&dir).map_err(|e|e.to_string())?;
 let safe_id=hex::encode(Sha256::digest(profile_id.as_bytes()));
 let dest=dir.join(format!("{}.{}",safe_id,ext));
 fs::copy(source,&dest).map_err(|e|e.to_string())?;
 profile.cover_path=Some(dest.to_string_lossy().into_owned());
 save(&app,&l)?;Ok("Обкладинку збережено".into())
}
#[tauri::command]
fn read_cover(path:String)->Result<String,String>{
 let p=PathBuf::from(&path);
 let bytes=fs::read(&p).map_err(|e|e.to_string())?;
 if bytes.len()>12_000_000{return Err("Обкладинка завелика".into())}
 let mime=match p.extension().and_then(|x|x.to_str()).unwrap_or("").to_lowercase().as_str(){"png"=>"image/png","webp"=>"image/webp",_=>"image/jpeg"};
 Ok(format!("data:{};base64,{}",mime,base64::Engine::encode(&base64::engine::general_purpose::STANDARD,bytes)))
}


// Lightweight WAD directory parser: bounds-checked, no external executable required.
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
struct WadDetails { format:String, lumps:usize, maps:Vec<String>, titlepic:bool, is_iwad:bool }
fn inspect_wad_file(path:&Path)->Result<WadDetails,String>{
 let b=fs::read(path).map_err(|e|e.to_string())?;
 if b.len()<12{return Err("Not a valid WAD header".into())}
 let magic=&b[..4];if magic!=b"IWAD" && magic!=b"PWAD"{return Err("Unsupported WAD signature".into())}
 let count=u32::from_le_bytes(b[4..8].try_into().unwrap()) as usize;
 let offset=u32::from_le_bytes(b[8..12].try_into().unwrap()) as usize;
 if count>100_000 || offset.checked_add(count.checked_mul(16).ok_or("Invalid WAD directory")?).is_none_or(|end|end>b.len()){return Err("Invalid WAD directory".into())}
 let mut maps=Vec::new();let mut titlepic=false;
 for i in 0..count{
  let at=offset+i*16;let name=String::from_utf8_lossy(&b[at+8..at+16]).trim_end_matches('\0').to_ascii_uppercase();
  if name=="TITLEPIC"{titlepic=true}
  let bytes=name.as_bytes();
  let doom2=bytes.len()==5 && bytes.starts_with(b"MAP") && bytes[3..].iter().all(|c|c.is_ascii_digit());
  let doom1=bytes.len()==4 && bytes[0]==b'E' && bytes[2]==b'M' && bytes[1].is_ascii_digit() && bytes[3].is_ascii_digit();
  if doom1||doom2{maps.push(name)}
 }
 maps.sort();maps.dedup();
 Ok(WadDetails{format:String::from_utf8_lossy(magic).to_string(),lumps:count,maps,titlepic,is_iwad:magic==b"IWAD"})
}
#[tauri::command]
fn inspect_wad(path:String)->Result<WadDetails,String>{inspect_wad_file(Path::new(&path))}

// Portable profile archive: original commercial IWAD and source-port EXE are never redistributed.
#[derive(Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
struct ProfileBundle {format_version:u32,profile:Profile,iwad_sha256:String,mods:Vec<ModFile>,maps:Vec<ModFile>}
#[tauri::command]
fn export_profile(app:tauri::AppHandle,id:String,path:String)->Result<String,String>{
 use std::io::Write;
 let l=load(&app)?;let profile=l.profiles.iter().find(|p|p.id==id).ok_or("Profile not found")?.clone();
 let game=l.games.iter().find(|g|g.id==profile.game_id).ok_or("IWAD not found")?;
 let mods=profile.mod_ids.iter().map(|id|l.mods.iter().find(|m|&m.id==id).cloned().ok_or("Missing mod".to_string())).collect::<Result<Vec<_>,_>>()?;
 let maps=profile.map_ids.iter().map(|id|l.maps.iter().find(|m|&m.id==id).cloned().ok_or("Missing map".to_string())).collect::<Result<Vec<_>,_>>()?;
 let manifest=ProfileBundle{format_version:1,profile:profile.clone(),iwad_sha256:game.sha256.clone(),mods:mods.clone(),maps:maps.clone()};
 let file=fs::File::create(&path).map_err(|e|e.to_string())?;
 let mut z=zip::ZipWriter::new(file);let opts=zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
 z.start_file("manifest.json",opts).map_err(|e|e.to_string())?;
 z.write_all(&serde_json::to_vec_pretty(&manifest).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
 for (folder,items) in [("mods",mods),("maps",maps)]{
  for m in items{
   let bytes=fs::read(&m.path).map_err(|e|e.to_string())?;
   if hex::encode(Sha256::digest(&bytes))!=m.id{return Err(format!("File changed since import: {}",m.name))}
   z.start_file(format!("{folder}/{}",m.id),opts).map_err(|e|e.to_string())?;
   z.write_all(&bytes).map_err(|e|e.to_string())?;
  }
 }
 if let Some(cover)=&profile.cover_path{
  let source=Path::new(cover);if source.is_file(){let ext=source.extension().and_then(|x|x.to_str()).unwrap_or("png");if ["png","jpg","jpeg","webp"].contains(&ext){z.start_file(format!("cover.{ext}"),opts).map_err(|e|e.to_string())?;z.write_all(&fs::read(source).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;}}
 }
 z.finish().map_err(|e|e.to_string())?;Ok(path)
}
#[tauri::command]
fn import_profile_bundle(app:tauri::AppHandle,path:String)->Result<Library,String>{
 use std::io::Read;
 let f=fs::File::open(path).map_err(|e|e.to_string())?;
 let mut zip=zip::ZipArchive::new(f).map_err(|e|e.to_string())?;
 let mut json=String::new();zip.by_name("manifest.json").map_err(|e|e.to_string())?.take(1_000_000).read_to_string(&mut json).map_err(|e|e.to_string())?;
 let bundle:ProfileBundle=serde_json::from_str(&json).map_err(|e|e.to_string())?;
 if bundle.format_version!=1{return Err("Unsupported bundle version".into())}
 let mut l=load(&app)?;
 let game=l.games.iter().find(|g|g.sha256==bundle.iwad_sha256).ok_or("Import matching IWAD into Games first")?;
 let game_id=game.id.clone();
 // Avoid partial imports for missing or corrupt entries.
 let mut staged=Vec::new();
 for (folder,items) in [("mods",&bundle.mods),("maps",&bundle.maps)]{
  for m in items{
   if m.id.len()!=64 || !m.id.bytes().all(|c|c.is_ascii_hexdigit()){return Err("Invalid mod hash".into())}
   let mut file=zip.by_name(&format!("{folder}/{}",m.id)).map_err(|e|e.to_string())?;
   if file.size()>512*1024*1024{return Err("Archive entry exceeds 512 MB".into())}
   let mut bytes=Vec::new();file.read_to_end(&mut bytes).map_err(|e|e.to_string())?;
   if hex::encode(Sha256::digest(&bytes))!=m.id{return Err("Archive checksum mismatch".into())}
   staged.push((folder.to_string(),m.clone(),bytes));
  }
 }
 for (folder,m,bytes) in staged{
  let dir=data_path(&app)?.join(&folder);fs::create_dir_all(&dir).map_err(|e|e.to_string())?;
  let dest=dir.join(&m.id);if !dest.exists(){fs::write(&dest,bytes).map_err(|e|e.to_string())?}
  let mut imported=m;imported.path=dest.to_string_lossy().into_owned();
  let target=if folder=="mods"{&mut l.mods}else{&mut l.maps};if !target.iter().any(|x|x.id==imported.id){target.push(imported)}
 }
 let mut profile=bundle.profile;profile.id=format!("import-{}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos());
 profile.game_id=game_id;profile.engine_path=String::new();profile.launches=0;profile.play_seconds=0;profile.cover_path=None;
 for i in 0..zip.len(){let mut file=zip.by_index(i).map_err(|e|e.to_string())?;let name=file.name().to_string();if name.starts_with("cover.") && ["png","jpg","jpeg","webp"].contains(&&name[6..]) && file.size()<=12_000_000{let dest=data_path(&app)?.join("covers").join(format!("{}.{}",profile.id,&name[6..]));fs::create_dir_all(dest.parent().unwrap()).map_err(|e|e.to_string())?;let mut output=fs::File::create(&dest).map_err(|e|e.to_string())?;std::io::copy(&mut file,&mut output).map_err(|e|e.to_string())?;profile.cover_path=Some(dest.to_string_lossy().into_owned());break}}
 l.profiles.push(profile);save(&app,&l)?;Ok(l)
}


#[derive(Serialize)]
#[serde(rename_all="camelCase")]
struct AchievementStats { total_seconds:u64, total_launches:u64, successful_sessions:u64, distinct_played:u64, completed_profiles:Vec<String>, sessions:u64 }
#[tauri::command]
fn get_achievement_stats(app:tauri::AppHandle)->Result<AchievementStats,String>{
 let conn=database(&app)?;
 let (seconds,sessions,successful,distinct):(i64,i64,i64,i64)=conn.query_row(
  "SELECT COALESCE(SUM(seconds),0),COUNT(*),COALESCE(SUM(CASE WHEN exit_code=0 THEN 1 ELSE 0 END),0),COUNT(DISTINCT profile_id) FROM play_history",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(|e|e.to_string())?;
 let mut stmt=conn.prepare("SELECT profile_id FROM completed_profiles ORDER BY completed_at").map_err(|e|e.to_string())?;
 let completed=stmt.query_map([],|r|r.get::<_,String>(0)).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
 Ok(AchievementStats{total_seconds:seconds.max(0) as u64,total_launches:sessions.max(0) as u64,successful_sessions:successful.max(0) as u64,distinct_played:distinct.max(0) as u64,completed_profiles:completed,sessions:sessions.max(0) as u64})
}
#[tauri::command]
fn set_profile_completed(app:tauri::AppHandle,profile_id:String,completed:bool)->Result<AchievementStats,String>{
 let library=load(&app)?;
 if !library.profiles.iter().any(|p|p.id==profile_id){return Err("Profile not found".into())}
 let conn=database(&app)?;
 if completed {conn.execute("INSERT OR IGNORE INTO completed_profiles(profile_id) VALUES(?1)",[&profile_id]).map_err(|e|e.to_string())?;}
 else {conn.execute("DELETE FROM completed_profiles WHERE profile_id=?1",[&profile_id]).map_err(|e|e.to_string())?;}
 get_achievement_stats(app)
}


/// Generate a map with a separately installed OBSIDIAN and import it into the map library.
/// Runs from the OBSIDIAN installation directory so its CONFIG.txt and data files resolve.
#[tauri::command]
async fn generate_obsidian_map(app:tauri::AppHandle,exe_path:String,name:String,settings:std::collections::HashMap<String,String>)->Result<Library,String>{
 tauri::async_runtime::spawn_blocking(move||{
  let exe=PathBuf::from(&exe_path);
  if !exe.is_file() || !exe.extension().is_some_and(|e|e.eq_ignore_ascii_case("exe")) {return Err("Select an existing OBSIDIAN .exe".into())}
  let home=exe.parent().ok_or("Invalid OBSIDIAN path")?;
  let safe_name:String=name.chars().filter(|c|c.is_ascii_alphanumeric()||*c=='-'||*c=='_').take(60).collect();
  let safe_name=if safe_name.is_empty(){"generated".to_owned()}else{safe_name};
  let out_dir=data_path(&app)?.join("generated");fs::create_dir_all(&out_dir).map_err(|e|e.to_string())?;
  let out=out_dir.join(format!("{}-{}.wad",safe_name,std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e|e.to_string())?.as_millis()));
  // CLI supports updating config keys before a batch build. Avoid modifying user CONFIG.txt:
  // copy installation config to an isolated work folder and launch with that working directory.
  // OBSIDIAN needs its relative resource directories, so use its own working directory and
  // restore CONFIG.txt even when a subprocess fails.
  let config=home.join("CONFIG.txt");
  let original=fs::read(&config).ok();
  struct ConfigRestore{path:PathBuf,original:Option<Vec<u8>>}
  impl Drop for ConfigRestore{fn drop(&mut self){if let Some(ref data)=self.original{let _=fs::write(&self.path,data);}else{let _=fs::remove_file(&self.path);}}}
  let _restore=ConfigRestore{path:config.clone(),original};
  // Only documented top-level config keys, with strictly validated values.
  let allowed:[(&str,&[&str]);18]=[
    ("game", &["doom1","doom2","tnt","plutonia"]),
    ("length", &["single","few","episode","game"]),
    ("theme", &["original"]),
    ("outdoors", &["none","few","some","heaps","mixed"]),
    ("bosses", &["none","minor","medium","mixed"]),
    ("caves", &["none","few","some","heaps","mixed"]),
    ("liquids", &["none","few","some","heaps","mixed"]),
    ("hallways", &["none","few","some","heaps","mixed"]),
    ("teleporters", &["none","few","some","heaps","mixed"]),
    ("steepness", &["none","few","some","heaps","mixed"]),
    ("secrets", &["none","few","some","heaps","mixed"]),
    ("health", &["scarce","less","normal","more","heaps"]),
    ("ammo", &["scarce","less","normal","more","heaps"]),
    ("weapons", &["scarce","less","normal","more","heaps"]),
    ("items", &["scarce","less","normal","more","heaps"]),
    ("float_size", &["20","36","55","75"]),
    ("float_mons", &["0.5","1","1.5","2"]),
    ("float_strength", &["0.5","1","1.5","2"]),
  ];
  let mut config_text=fs::read_to_string(&config).unwrap_or_default();
  for (key,values) in allowed{
    if let Some(value)=settings.get(key){
      if !values.contains(&value.as_str()){return Err(format!("Invalid OBSIDIAN setting: {key}={value}"))}
      let mut found=false;
      let mut lines:Vec<String>=config_text.lines().map(|line|{
        if line.trim_start().starts_with(&format!("{key} =")) {found=true;format!("{key} = {value}")}else{line.to_owned()}
      }).collect();
      if !found{lines.push(format!("{key} = {value}"));}
      config_text=lines.join("\n")+"\n";
    }
  }
  // OBSIDIAN v20 can write a ZIP/PK3 container even when the output path ends in .wad.
  // Request uncompressed WAD output, but still recognize ZIP as a valid map package.
  if config_text.lines().any(|l|l.trim_start().starts_with("@compress_output =")) {
    config_text=config_text.lines().map(|l|if l.trim_start().starts_with("@compress_output =") {"@compress_output = 0"} else {l}).collect::<Vec<_>>().join("\n")+"\n";
  } else {config_text.push_str("\n@compress_output = 0\n");}
  fs::write(&config,&config_text).map_err(|e|format!("Cannot update OBSIDIAN CONFIG.txt: {e}"))?;
  let result=Command::new(&exe).arg("--batch").arg(&out).current_dir(home).output().map_err(|e|format!("Could not launch OBSIDIAN: {e}"))?;
  let stderr=String::from_utf8_lossy(&result.stderr);
  let stdout=String::from_utf8_lossy(&result.stdout);
  if !result.status.success(){return Err(format!("OBSIDIAN exited with {:?}. {} {}. See {}",result.status.code(),stderr.chars().take(350).collect::<String>(),stdout.chars().take(350).collect::<String>(),home.join("LOGS.txt").display()))}
  let header=fs::read(&out).map_err(|e|format!("OBSIDIAN finished but output cannot be read: {e}. Check {}",home.join("LOGS.txt").display()))?;
  let is_wad=header.len()>=12 && (&header[..4]==b"PWAD" || &header[..4]==b"IWAD");
  let is_zip=header.len()>=4 && (&header[..4]==b"PK\x03\x04" || &header[..4]==b"PK\x05\x06");
  if !is_wad && !is_zip{return Err(format!("OBSIDIAN output has unsupported header {:?} ({} bytes) at {}. See {}",&header[..header.len().min(8)],header.len(),out.display(),home.join("LOGS.txt").display()))}
  let import_path=if is_zip {let pk3=out.with_extension("pk3");fs::rename(&out,&pk3).map_err(|e|format!("Could not rename compressed map to PK3: {e}"))?;pk3}else{out.clone()};
  let imported=import_mod(app.clone(),import_path.to_string_lossy().into_owned(),Some("map".into()));
  if imported.is_ok(){let _=fs::remove_file(&import_path);}
  imported
 }).await.map_err(|e|e.to_string())?
}
pub fn run(){tauri::Builder::default().plugin(tauri_plugin_dialog::init()).invoke_handler(tauri::generate_handler![rename_library_item,reveal_library_folder,generate_obsidian_map,get_achievement_stats,set_profile_completed,get_library,backup_library,restore_library,get_play_history,inspect_wad,export_profile,import_profile_bundle,import_game,save_steamgrid_key,fetch_profile_cover,refresh_steamgrid_covers,refresh_game_covers,save_game_cover,register_engine,unregister_engine,import_mod,delete_library_item,save_cover,read_cover,save_profile,delete_profile,launch_profile,list_engines,install_engine]).run(tauri::generate_context!()).expect("Hellforge failed");}
