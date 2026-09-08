-- Torii — migration 0003 : un relevé quotidien, pour voir l'évolution du service.
--
-- `schema.sql` décrit la base telle qu'on la créerait aujourd'hui ; ce fichier fait
-- passer une base DÉJÀ DÉPLOYÉE d'un état à l'autre. Les deux doivent rester d'accord.
--
-- Appliquer :
--   npx wrangler d1 execute torii --remote --file=migrations/0003_stats.sql
--
-- Rejouable sans dégât : `IF NOT EXISTS`.
--
-- 🔑 POURQUOI CETTE TABLE EXISTE. Rien n'enregistrait l'histoire du service : `presence`
-- expire, `accounts` ne fait que grossir. On pouvait donc connaître l'état à l'instant t,
-- jamais la pente — alors que c'est la pente qui dit s'il faut passer au plan payant, et
-- quand. Une ligne par jour, écrite par le cron : le coût est nul (24 écritures par jour
-- contre 100 000 offertes) et l'information, elle, ne se rattrape pas après coup.
--
-- ⚠️ Aucune donnée personnelle ici : que des COMPTES, jamais qui, jamais quoi. La
-- promesse « aucun historique » du README porte sur ce que les gens jouent ; elle n'est
-- pas entamée par un décompte anonyme.

CREATE TABLE IF NOT EXISTS stats (
  -- Jour UTC (AAAA-MM-JJ), pas heure de Paris : une bascule d'heure d'été ne doit pas
  -- créer deux lignes pour le même jour, ni en effacer une.
  jour         TEXT PRIMARY KEY,

  -- Cumuls, vrais à n'importe quelle heure.
  comptes      INTEGER NOT NULL,
  biblios      INTEGER NOT NULL,

  -- Comptes ayant eu une session vue dans les 7 derniers jours. C'est la mesure d'usage
  -- réel : « 40 comptes créés » ne dit pas combien s'en servent encore.
  actifs_7j    INTEGER NOT NULL,

  -- 🔑 Le PIC de présence simultanée du jour, pas la valeur à l'instant du relevé.
  -- C'est ce chiffre-là, et lui seul, qui se compare au plafond du service : relever la
  -- présence une fois par nuit donnerait zéro tous les jours et ne préviendrait de rien.
  -- D'où le relevé horaire, qui ne fait que remonter ce maximum.
  pic_en_ligne INTEGER NOT NULL,

  releve_at    INTEGER NOT NULL
);
