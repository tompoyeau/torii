/**
 * Révèle les blocs à mesure qu'ils entrent dans l'écran.
 *
 * 🔑 POURQUOI LA CLASSE `js` EST POSÉE ICI, ET EN PREMIER. Les règles qui masquent les
 * blocs sont toutes préfixées par `.js` dans la feuille de style. Tant que ce fichier
 * n'a pas tourné, rien n'est masqué : une erreur de script, un navigateur trop ancien,
 * un bloqueur trop zélé — et la page reste entièrement lisible au lieu de se présenter
 * vide. C'est la seule façon sûre de faire de l'apparition au défilement.
 *
 * ⚠️ Ce fichier est chargé en fin de `<body>`, donc APRÈS le premier rendu : les blocs
 * du haut de page sont déjà peints quand `js` arrive. Ils sont donc révélés
 * immédiatement, sans transition, par le premier passage de l'observateur — c'est voulu,
 * l'en-tête a sa propre animation d'entrée en CSS.
 */
document.documentElement.classList.add("js");

const BLOCS = ".section-texte, .section-illu, .bandeau, .comparaison h2, .comparaison .intro, .grille-comparaison, .faq h2, .faq details, .essai-texte, .essai-illu, .final";

const cibles = document.querySelectorAll(BLOCS);
for (const el of cibles) el.classList.add("reveal");

// Sans IntersectionObserver (navigateurs anciens), on montre tout d'un coup : mieux vaut
// une page sans effet qu'une page dont la moitié ne s'affiche jamais.
if (!("IntersectionObserver" in window)) {
  for (const el of cibles) el.classList.add("vu");
} else {
  const guetteur = new IntersectionObserver(
    (entrees) => {
      for (const e of entrees) {
        if (!e.isIntersecting) continue;
        e.target.classList.add("vu");
        // Une seule fois : un bloc qui disparaît et réapparaît à chaque va-et-vient du
        // défilement est fatigant, et rejoue la cascade à contretemps.
        guetteur.unobserve(e.target);
      }
    },
    // Marge négative en bas : le bloc se révèle quand il est franchement entré dans
    // l'écran, pas à l'instant où son premier pixel affleure.
    { rootMargin: "0px 0px -12% 0px", threshold: 0.08 },
  );
  for (const el of cibles) guetteur.observe(el);
}
