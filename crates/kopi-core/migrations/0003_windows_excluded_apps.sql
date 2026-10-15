-- The default excluded apps on Windows: password managers. Most of them already mark their
-- copies with ExcludeClipboardContentFromMonitorProcessing, so this is a second layer, as on
-- macOS. Seeded once, so a default the user removes stays removed.
--
-- Windows apps are known by their exe file name, matched without regard to case.
INSERT OR IGNORE INTO excluded_apps (platform, app_id, app_name, origin, created_at)
VALUES
  ('windows', '1Password.exe',  '1Password', 'default', CAST(unixepoch('subsec') * 1000 AS INTEGER)),
  ('windows', 'Bitwarden.exe',  'Bitwarden', 'default', CAST(unixepoch('subsec') * 1000 AS INTEGER)),
  ('windows', 'KeePass.exe',    'KeePass',   'default', CAST(unixepoch('subsec') * 1000 AS INTEGER)),
  ('windows', 'KeePassXC.exe',  'KeePassXC', 'default', CAST(unixepoch('subsec') * 1000 AS INTEGER)),
  ('windows', 'Enpass.exe',     'Enpass',    'default', CAST(unixepoch('subsec') * 1000 AS INTEGER));
