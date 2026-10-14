/**
 * Fills `{placeholder}` slots in an i18n template.
 *
 * `String.prototype.replace` with a *string* replacement treats `$&`, `$`,
 * `` $` `` and `$'` as special sequences, so a task literally named
 * "Купить $& хлеб" would corrupt its own aria-label. Passing a function makes
 * the substitution literal.
 *
 * @example fill('Завершить задачу «{title}»', 'Отчёт') // → 'Завершить задачу «Отчёт»'
 */
export function fill(template: string, placeholder: string, value: string): string {
  return template.replace(`{${placeholder}}`, () => value);
}