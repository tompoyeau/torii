-- Torii — schéma du service social (Cloudflare D1 / SQLite).
--
-- Appliquer : npx wrangler d1 execute torii --remote --file=schema.sql
--
-- Principes tenus par ce schéma :
--   * Aucun mot de passe n'est stocké : la connexion se fait par code à usage unique
--     envoyé par e-mail (voir src/auth.js).
--   * Rien de secret n'est stocké en clair : codes de connexion et jetons de session
--     ne sont conservés que hachés (SHA-256 + poivre serveur).
--   * La présence est éphémère par construction : chaque ligne porte sa date de
--     péremption et n'est jamais archivée. Il n'existe donc AUCUN historique de ce que
--     les gens jouent, et c'est volontaire.

-- Un compte Torii. L'e-mail est l'identité ; il est normalisé (minuscules, sans espaces).
CREATE TABLE IF NOT EXISTS accounts (
  id                 TEXT PRIMARY KEY,
  email              TEXT NOT NULL UNIQUE,
  display_name       TEXT NOT NULL,
  -- Code court partagé de la main à la main pour se faire ajouter. C'est le SEUL moyen
  -- d'ajouter quelqu'un : inviter par e-mail permettrait de tester une liste d'adresses
  -- pour découvrir qui utilise Torii.
  friend_code        TEXT NOT NULL UNIQUE,
  -- SteamID associé (facultatif), pour suggérer des amis Steam déjà sur Torii.
  steam_id           TEXT,
  -- 🔑 La suggestion par Steam n'a lieu QUE si les deux personnes l'ont activée.
  steam_discoverable INTEGER NOT NULL DEFAULT 0,
  created_at         INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS accounts_steam ON accounts(steam_id) WHERE steam_id IS NOT NULL;

-- Code de connexion en cours de validité (un seul à la fois par e-mail).
CREATE TABLE IF NOT EXISTS login_codes (
  email      TEXT PRIMARY KEY,
  code_hash  TEXT NOT NULL,
  expires_at INTEGER NOT NULL,
  -- Nombre d'essais ratés : au-delà de MAX_ATTEMPTS le code est brûlé, ce qui empêche
  -- de deviner 6 chiffres par force brute.
  attempts   INTEGER NOT NULL DEFAULT 0,
  sent_at    INTEGER NOT NULL,
  -- Envois dans la fenêtre courante. Sans ce compteur, attendre le délai minimal entre
  -- deux envois suffirait à inonder la boîte mail de quelqu'un d'autre indéfiniment.
  -- La ligne disparaît dès qu'une connexion réussit : un utilisateur légitime ne le voit
  -- jamais, seul quelqu'un qui n'arrive pas à se connecter fait monter le compteur.
  sends        INTEGER NOT NULL DEFAULT 1,
  window_start INTEGER NOT NULL
);

-- Sessions ouvertes. Une par appareil : le PC et (plus tard) le mobile coexistent, et
-- on peut en révoquer une sans toucher aux autres.
CREATE TABLE IF NOT EXISTS sessions (
  token_hash   TEXT PRIMARY KEY,
  account_id   TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
  device       TEXT,
  created_at   INTEGER NOT NULL,
  last_seen_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS sessions_account ON sessions(account_id);

-- Relation d'amitié, toujours réciproque une fois acceptée. La paire est stockée une
-- seule fois (demandeur, destinataire) ; les lectures interrogent les deux colonnes.
CREATE TABLE IF NOT EXISTS friendships (
  requester_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
  addressee_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
  state        TEXT NOT NULL CHECK (state IN ('pending', 'accepted')),
  created_at   INTEGER NOT NULL,
  PRIMARY KEY (requester_id, addressee_id)
);
CREATE INDEX IF NOT EXISTS friendships_addressee ON friendships(addressee_id, state);

-- Présence courante. Une ligne par compte, écrasée à chaque battement de cœur.
-- `expires_at` fait tout le travail : plus de battement (Torii fermé, PC endormi,
-- réseau coupé) et la personne devient hors ligne toute seule.
CREATE TABLE IF NOT EXISTS presence (
  account_id TEXT PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
  status     TEXT NOT NULL CHECK (status IN ('in-game', 'online', 'away')),
  -- Clé de jeu cross-launcher : « igdb:1942 » quand on la connaît, sinon
  -- « title:<titre normalisé> ». C'est ce qui permet de reconnaître le même jeu
  -- entre un ami sur GOG et un autre sur Steam.
  game_key   TEXT,
  game_title TEXT,
  since      INTEGER,
  expires_at INTEGER NOT NULL
);

-- À VENIR (application mobile) : synchronisation de la bibliothèque, PC → serveur →
-- mobile. Volontairement absente pour l'instant — c'est une donnée durable et
-- volumineuse, à l'opposé de la présence, et elle demandera sa propre migration
-- (table `libraries` versionnée par appareil source, pour que deux PC ne s'effacent
-- pas mutuellement).
