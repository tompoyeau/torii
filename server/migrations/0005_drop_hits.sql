-- L'audience du site vitrine passe par Umami (stats.topo-host.com) depuis le 8 octobre 2026 :
-- l'ancien compteur maison (`POST /v1/hit`) a été retiré avec le panneau `/admin`, et plus
-- rien n'écrit ni ne purge cette table. Ses 21 visites (26/09 → 06/10) sont abandonnées.
DROP INDEX IF EXISTS hits_jour;
DROP TABLE IF EXISTS hits;
