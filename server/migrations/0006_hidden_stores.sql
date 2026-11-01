-- Torii — migration 0006 : les revendeurs masqués suivent le compte.
--
-- Appliquer AVANT de déployer le Worker : `authenticate` lit cette colonne à chaque requête
-- connectée, et sans elle, plus personne ne pourrait se servir de son compte.
--   npx wrangler d1 execute torii --remote --file=migrations/0006_hidden_stores.sql
--
-- L'ajout échoue avec « duplicate column name » si la colonne est déjà là (sans conséquence).

ALTER TABLE accounts ADD COLUMN hidden_stores TEXT NOT NULL DEFAULT '[]';
