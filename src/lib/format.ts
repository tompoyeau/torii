/** Formate un montant en euros, format français : `44,99 €`. */
export function formatEur(n: number): string {
  return `${n.toLocaleString("fr-FR", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  })} €`;
}
