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
  -- Les amis peuvent-ils consulter ma bibliothèque ? Éteint par défaut, comme le partage
  -- de présence. ⚠️ À ne pas confondre avec le fait de SYNCHRONISER : synchroniser sert à
  -- retrouver sa bibliothèque sur son propre mobile et ne dépend pas de ce drapeau ; ce
  -- drapeau n'ouvre que la lecture par les amis (voir src/library.js).
  share_library      INTEGER NOT NULL DEFAULT 0,
  created_at         INTEGER NOT NULL
);
-- 🔑 UNIQUE, et pas seulement un index de recherche. Deux comptes Torii portant le même
-- SteamID rendaient les suggestions ambiguës (deux propositions pour une personne), la
-- fusion côté client arbitraire (le premier arrivé absorbe l'identité Steam, le second
-- s'affiche en double et paraît mort) et la présence contradictoire. Le contrôle est fait
-- dans `updateMe`, mais il est ici aussi : deux requêtes simultanées passeraient à travers
-- un contrôle applicatif seul.
CREATE UNIQUE INDEX IF NOT EXISTS accounts_steam ON accounts(steam_id) WHERE steam_id IS NOT NULL;

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
--
-- 🔑 `last_seen_at` n'est pas décoratif : c'est lui qui fait expirer une session dormante
-- (SESSION_TTL dans auth.js) et qui alimente la liste « mes appareils ». Il n'est
-- rafraîchi qu'une fois par jour et par appareil — le client bat le cœur toutes les 30 s,
-- l'écrire à chaque requête consommerait à lui seul tout le quota d'écritures D1.
CREATE TABLE IF NOT EXISTS sessions (
  token_hash   TEXT PRIMARY KEY,
  -- Identifiant PUBLIC de l'appareil, montré dans la liste et cité pour le déconnecter.
  -- Distinct de l'empreinte, qui est un secret dérivé et ne sort jamais du serveur.
  id           TEXT,
  account_id   TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
  device       TEXT,
  created_at   INTEGER NOT NULL,
  last_seen_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS sessions_account ON sessions(account_id);
CREATE UNIQUE INDEX IF NOT EXISTS sessions_id ON sessions(id) WHERE id IS NOT NULL;

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

-- Index des bibliothèques synchronisées. 🔑 La liste des jeux n'est PAS ici : elle vit
-- dans R2, un objet JSON par appareil (`lib/<compte>/<appareil>.json`). Une ligne par jeu
-- et par personne, c'est ~1 000 écritures par resynchronisation contre 100 000 par jour
-- offertes — le service s'arrêterait à une centaine de joueurs. Cette table ne garde donc
-- que de quoi savoir, SANS rien télécharger, si une bibliothèque a changé.
--
-- ⚠️ Première donnée durable du service : la promesse « aucun historique » ne concerne
-- que la présence. Ici on sait ce que les gens possèdent — jamais ce qu'ils jouent.
--
-- Une ligne par APPAREIL : deux PC n'ont pas la même bibliothèque, et s'ils écrivaient au
-- même endroit le dernier passé effacerait l'autre indéfiniment.
CREATE TABLE IF NOT EXISTS libraries (
  account_id  TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
  device_id   TEXT NOT NULL,
  device_name TEXT NOT NULL,
  -- Empreinte opaque calculée par le client : elle sert d'ETag à la lecture (304 sans
  -- lecture R2 ni transfert) et évite de renvoyer une bibliothèque qui n'a pas bougé.
  digest      TEXT NOT NULL,
  game_count  INTEGER NOT NULL,
  size_bytes  INTEGER NOT NULL,
  updated_at  INTEGER NOT NULL,
  PRIMARY KEY (account_id, device_id)
);
-- Pas d'index supplémentaire : la clé primaire commence par `account_id`, donc « les
-- appareils de cette personne » est déjà une recherche par préfixe.

-- Relevé quotidien du service : une ligne par jour UTC, écrite par le cron. Rien
-- n'enregistrait l'histoire (la présence expire, les comptes ne font que grossir), donc
-- on connaissait l'état sans jamais connaître la pente — or c'est la pente qui dit s'il
-- faut passer au plan payant, et quand. Que des décomptes : jamais qui, jamais quoi.
CREATE TABLE IF NOT EXISTS stats (
  jour         TEXT PRIMARY KEY,   -- AAAA-MM-JJ en UTC
  comptes      INTEGER NOT NULL,
  biblios      INTEGER NOT NULL,
  actifs_7j    INTEGER NOT NULL,   -- comptes vus dans les 7 derniers jours
  -- 🔑 Le PIC du jour, pas la valeur au moment du relevé : c'est lui qui se compare au
  -- plafond de présence simultanée. Une mesure unique par nuit vaudrait zéro.
  pic_en_ligne INTEGER NOT NULL,
  releve_at    INTEGER NOT NULL
);
