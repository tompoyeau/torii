-- Torii — migration 0001 : synchronisation des bibliothèques (R2 + index D1).
--
-- `schema.sql` décrit la base telle qu'on la créerait aujourd'hui ; ce fichier fait
-- passer une base DÉJÀ DÉPLOYÉE d'un état à l'autre. Les deux doivent rester d'accord.
--
-- Appliquer :
--   npx wrangler d1 execute torii --remote --file=migrations/0001_libraries.sql
--
-- Rejouable sans dégât : `IF NOT EXISTS` pour la table, et l'ajout de colonne échoue
-- avec « duplicate column name » si elle est déjà là — message sans conséquence.

-- Les amis peuvent-ils consulter ma bibliothèque ? Éteint par défaut : un compte existant
-- ne se met pas à partager parce qu'on a déployé une version.
ALTER TABLE accounts ADD COLUMN share_library INTEGER NOT NULL DEFAULT 0;

CREATE TABLE IF NOT EXISTS libraries (
  account_id  TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
  device_id   TEXT NOT NULL,
  device_name TEXT NOT NULL,
  digest      TEXT NOT NULL,
  game_count  INTEGER NOT NULL,
  size_bytes  INTEGER NOT NULL,
  updated_at  INTEGER NOT NULL,
  PRIMARY KEY (account_id, device_id)
);
