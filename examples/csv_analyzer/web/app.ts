type RowError = { line: number; code: string };
type Report = { rows: number; accepted: number; quantity: number; total_cents: number; errors: RowError[] };

const form = document.querySelector<HTMLFormElement>('#analyze')!;
const csv = document.querySelector<HTMLTextAreaElement>('#csv')!;
const file = document.querySelector<HTMLInputElement>('#file')!;
const submit = document.querySelector<HTMLButtonElement>('#submit')!;
const status = document.querySelector<HTMLElement>('#status')!;
const results = document.querySelector<HTMLElement>('#results')!;
const errors = document.querySelector<HTMLTableSectionElement>('#errors')!;

const messages: Record<string, string> = {
  expected_header: 'Use the header sku,quantity,unit_cents.',
  too_many_rows: 'Use at most 100 data rows.',
  csv_too_large: 'Use a CSV file of at most 8 KiB.',
  expected_ascii_csv: 'This example accepts printable ASCII with LF or CRLF line endings.',
  invalid_line_ending: 'Use LF or CRLF line endings.',
  quoted_fields_unsupported: 'Quoted fields are outside this example’s CSV dialect.',
  expected_three_columns: 'Expected three comma-separated columns.',
  missing_sku: 'SKU is required.',
  invalid_quantity: 'Quantity must be an integer from 1 to 1,000,000.',
  invalid_unit_cents: 'Unit cents must be an integer from 0 to 1,000,000.',
  backend_unavailable: 'Cannot reach the Veln backend. Check that it is running and try again.',
};

let revision = 0;

function clearResult() {
  revision += 1;
  results.hidden = true;
  errors.replaceChildren();
  status.textContent = '';
}

csv.addEventListener('input', clearResult);
file.addEventListener('change', async () => {
  clearResult();
  const selected = file.files?.[0];
  if (!selected) return;
  if (selected.size > 8192) {
    status.textContent = messages.csv_too_large;
    file.value = '';
    return;
  }
  csv.value = await selected.text();
  clearResult();
});

document.querySelector<HTMLButtonElement>('#sample')!.addEventListener('click', async () => {
  clearResult();
  try {
    const response = await fetch('/sample.csv');
    if (!response.ok) throw new Error('Sample unavailable. Reload and try again.');
    csv.value = await response.text();
    clearResult();
    file.value = '';
  } catch (error) {
    status.textContent = error instanceof Error ? error.message : 'Could not load sample.';
  }
});

form.addEventListener('submit', async event => {
  event.preventDefault();
  clearResult();
  if (new TextEncoder().encode(csv.value).length > 8192) {
    status.textContent = messages.csv_too_large;
    return;
  }
  const submittedRevision = revision;
  submit.disabled = true;
  status.textContent = 'Analyzing…';
  try {
    const response = await fetch('/api/analyze', {
      method: 'POST', headers: { 'content-type': 'text/csv' }, body: csv.value,
      signal: AbortSignal.timeout(10000),
    });
    const payload = await response.json();
    if (revision !== submittedRevision) return;
    if (!response.ok) throw new Error(messages[payload.error] ?? `Request failed (${response.status}).`);
    const report = payload as Report;
    const values: Record<string, string> = {
      rows: String(report.rows), accepted: String(report.accepted),
      quantity: String(report.quantity), total: (report.total_cents / 100).toFixed(2),
    };
    for (const [id, value] of Object.entries(values)) document.getElementById(id)!.textContent = value;
    for (const error of report.errors) {
      const row = document.createElement('tr');
      for (const value of [String(error.line), messages[error.code] ?? error.code]) {
        const cell = document.createElement('td');
        cell.textContent = value;
        row.append(cell);
      }
      errors.append(row);
    }
    results.hidden = false;
    status.textContent = report.errors.length === 0 ? 'All rows accepted.' : `${report.errors.length} row(s) excluded from totals. Fix the rows below and analyze again.`;
  } catch (error) {
    if (revision === submittedRevision) status.textContent = error instanceof Error ? error.message : 'Analysis failed. Try again.';
  } finally {
    submit.disabled = false;
  }
});
