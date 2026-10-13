-- The default excluded apps: password managers and Keychain Access. Most of them already mark
-- their copies concealed, so this is a second layer. Seeded once, so a default the user removes
-- stays removed. See docs/PRIVACY_DESIGN.md.
--
-- Bundle IDs are matched without regard to case, as macOS does. Apps with more than one ID
-- (an older major version, or a Mac App Store build) get a row per ID.
INSERT OR IGNORE INTO excluded_apps (platform, app_id, app_name, origin, created_at)
VALUES
  ('macos', 'com.apple.keychainaccess',          'Keychain Access', 'default', CAST(unixepoch('subsec') * 1000 AS INTEGER)),
  ('macos', 'com.apple.Passwords',               'Passwords',       'default', CAST(unixepoch('subsec') * 1000 AS INTEGER)),
  ('macos', 'com.1password.1password',           '1Password',       'default', CAST(unixepoch('subsec') * 1000 AS INTEGER)),
  ('macos', 'com.agilebits.onepassword7',        '1Password 7',     'default', CAST(unixepoch('subsec') * 1000 AS INTEGER)),
  ('macos', 'com.bitwarden.desktop',             'Bitwarden',       'default', CAST(unixepoch('subsec') * 1000 AS INTEGER)),
  ('macos', 'org.keepassxc.keepassxc',           'KeePassXC',       'default', CAST(unixepoch('subsec') * 1000 AS INTEGER)),
  ('macos', 'com.dashlane.Dashlane',             'Dashlane',        'default', CAST(unixepoch('subsec') * 1000 AS INTEGER)),
  ('macos', 'com.dashlane.dashlanephonefinal',   'Dashlane',        'default', CAST(unixepoch('subsec') * 1000 AS INTEGER)),
  ('macos', 'in.sinew.Enpass-Desktop',           'Enpass',          'default', CAST(unixepoch('subsec') * 1000 AS INTEGER)),
  ('macos', 'in.sinew.Enpass-Desktop.App',       'Enpass',          'default', CAST(unixepoch('subsec') * 1000 AS INTEGER));
