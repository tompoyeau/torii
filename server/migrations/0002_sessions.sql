-- Torii — migration 0002 : les sessions deviennent visibles et périssables.
--
-- `schema.sql` décrit la base telle qu'on la créerait aujourd'hui ; ce fichier fait
-- passer une base DÉJÀ DÉPLOYÉE d'un état à l'autre. Les deux doivent rester d'accord.
--
-- Appliquer :
--   npx wrangler d1 execute torii --remote --file=migrations/0002_sessions.sql
--
-- Rejouable sans dégât : l'ajout de colonne échoue avec « duplicate column name » si elle
-- est déjà là (message sans conséquence), et les deux autres ordres sont idempotents.

-- Identifiant public d'un appareil : c'est lui qu'affiche « mes appareils » et qu'on cite
-- pour déconnecter une machine. L'empreinte du jeton, elle, ne sort jamais du serveur.
ALTER TABLE sessions ADD COLUMN id TEXT;

-- Les sessions déjà ouvertes n'en avaient pas. `randomblob` évite d'avoir à les relire
-- une par une depuis le Worker, et personne n'est déconnecté au passage.
UPDATE sessions SET id = lower(hex(randomblob(12))) WHERE id IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS sessions_id ON sessions(id) WHERE id IS NOT NULL;

-- ⚠️ PAS de purge des sessions anciennes ici, et vérifier avant de la faire un jour :
-- `last_seen_at` valait jusqu'à présent la date de CRÉATION, puisque rien ne le mettait à
-- jour. Une session très fréquentée y ressemble donc à une session abandonnée. Ça ne pose
-- aucun problème aujourd'hui — le réseau Torii date du 2026-08-21, aucune session n'a
-- deux semaines contre les six mois de SESSION_TTL, donc personne n'est déconnecté — mais
-- la colonne ne reprend son sens qu'après le premier passage de chaque client.
