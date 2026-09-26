-- Torii — migration 0004 : audience du site et relevé des téléchargements.
--
-- Appliquer : npx wrangler d1 execute torii --remote --file=migrations/0004_audience.sql

-- Une ligne par page vue (ou clic sur « Télécharger ») sur torii-app.fr.
--
-- 🔑 SANS COOKIE ET SANS SUIVI D'UN JOUR À L'AUTRE. `visiteur` est une empreinte de
-- (IP, navigateur, JOUR, secret du serveur) : elle permet de compter les visiteurs
-- distincts d'une journée, mais change à minuit — impossible de relier deux visites de
-- dates différentes, et l'IP elle-même n'est jamais stockée.
CREATE TABLE IF NOT EXISTS hits (
  at       INTEGER NOT NULL,             -- instant Unix
  jour     TEXT    NOT NULL,             -- AAAA-MM-JJ (UTC)
  type     TEXT    NOT NULL CHECK (type IN ('vue', 'telechargement')),
  page     TEXT    NOT NULL,             -- chemin seul, sans requête ni fragment
  langue   TEXT,                         -- langue de la page (fr / en)
  source   TEXT,                         -- domaine d'origine (référent), sans chemin
  pays     TEXT,                         -- code ISO donné par Cloudflare
  mobile   INTEGER NOT NULL DEFAULT 0,
  visiteur TEXT    NOT NULL
);
CREATE INDEX IF NOT EXISTS hits_jour ON hits(jour, type);

-- Cumuls de téléchargements GitHub, relevés une fois par jour.
--
-- GitHub ne donne que des TOTAUX depuis la publication, sans date : garder le relevé de
-- chaque jour est la seule façon d'obtenir « combien aujourd'hui » (par différence).
CREATE TABLE IF NOT EXISTS github_releve (
  jour        TEXT PRIMARY KEY,
  installeurs INTEGER NOT NULL,  -- .exe + .msi, toutes versions (installations ET mises à jour)
  controles   INTEGER NOT NULL   -- latest.json : vérifications de mise à jour par l'app
);
