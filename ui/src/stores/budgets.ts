import { writable } from 'svelte/store';
import type { TraceSummary } from '../lib/types';

const STORAGE_KEY = 'widescope:budgets';

export type BudgetField =
  | 'duration_ns'
  | 'cost_usd'
  | 'error_count'
  | 'llm_span_count'
  | 'span_count'
  | 'latency_p95_ns';

export type BudgetOperator = '>' | '>=' | '<' | '<=' | '==';

export interface Budget {
  id: string;
  field: BudgetField;
  operator: BudgetOperator;
  value: number;
}

export interface BudgetViolation {
  budget: Budget;
  observed: number | null;
}

export const BUDGET_FIELDS: { value: BudgetField; label: string; unit: string }[] = [
  { value: 'duration_ns', label: 'Total duration', unit: 'ms' },
  { value: 'latency_p95_ns', label: 'P95 latency', unit: 'ms' },
  { value: 'cost_usd', label: 'Total cost', unit: 'USD' },
  { value: 'error_count', label: 'Error count', unit: '' },
  { value: 'llm_span_count', label: 'LLM span count', unit: '' },
  { value: 'span_count', label: 'Span count', unit: '' },
];

export function fieldLabel(field: BudgetField): string {
  return BUDGET_FIELDS.find((f) => f.value === field)?.label ?? field;
}

export function fieldUnit(field: BudgetField): string {
  return BUDGET_FIELDS.find((f) => f.value === field)?.unit ?? '';
}

function loadAll(): Budget[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter(isBudget);
  } catch {
    return [];
  }
}

function isBudget(v: unknown): v is Budget {
  if (!v || typeof v !== 'object') return false;
  const b = v as Record<string, unknown>;
  return (
    typeof b.id === 'string' &&
    typeof b.field === 'string' &&
    typeof b.operator === 'string' &&
    typeof b.value === 'number'
  );
}

function persist(budgets: Budget[]) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(budgets));
  } catch {
    // quota exceeded — keep in-memory state
  }
}

function makeId(): string {
  return `b-${Date.now().toString(36)}-${Math.floor(Math.random() * 1e6).toString(36)}`;
}

function createBudgetsStore() {
  const initial = loadAll();
  const { subscribe, set, update } = writable<Budget[]>(initial);

  return {
    subscribe,

    add(field: BudgetField, operator: BudgetOperator, value: number): Budget {
      const budget: Budget = { id: makeId(), field, operator, value };
      update((list) => {
        const next = [...list, budget];
        persist(next);
        return next;
      });
      return budget;
    },

    update(id: string, patch: Partial<Omit<Budget, 'id'>>) {
      update((list) => {
        const next = list.map((b) => (b.id === id ? { ...b, ...patch } : b));
        persist(next);
        return next;
      });
    },

    remove(id: string) {
      update((list) => {
        const next = list.filter((b) => b.id !== id);
        persist(next);
        return next;
      });
    },

    clear() {
      set([]);
      persist([]);
    },
  };
}

export const budgets = createBudgetsStore();

/**
 * Evaluate a single budget against an observed value. Returns true when
 * the observation violates the budget (i.e. the inverse of the operator
 * holds — `field > value` is violated when the observed value is ≤ value).
 */
function isViolated(operator: BudgetOperator, observed: number, value: number): boolean {
  switch (operator) {
    case '>':
      return !(observed > value);
    case '>=':
      return !(observed >= value);
    case '<':
      return !(observed < value);
    case '<=':
      return !(observed <= value);
    case '==':
      return observed !== value;
  }
}

function observeField(field: BudgetField, summary: TraceSummary, totalCostUsd: number | null): number | null {
  switch (field) {
    case 'duration_ns':
      return summary.total_duration_ns;
    case 'latency_p95_ns':
      return parseLatencyDisplayNs(summary.latency_p95_display);
    case 'cost_usd':
      return totalCostUsd;
    case 'error_count':
      return summary.error_count;
    case 'llm_span_count':
      return summary.llm_span_count;
    case 'span_count':
      return summary.span_count;
  }
}

/**
 * Best-effort parse of a duration display string like "750.0ms" / "4.23s" / "120µs"
 * back into nanoseconds. Used because TraceSummary only ships the p95 as a
 * formatted string today; once we expose raw p95 ns, this can be removed.
 */
function parseLatencyDisplayNs(display: string): number | null {
  const m = display.match(/^([0-9]*\.?[0-9]+)\s*(ns|µs|us|ms|s|m)$/i);
  if (!m) return null;
  const value = parseFloat(m[1]);
  const unit = m[2].toLowerCase();
  const scale: Record<string, number> = {
    ns: 1,
    µs: 1_000,
    us: 1_000,
    ms: 1_000_000,
    s: 1_000_000_000,
    m: 60 * 1_000_000_000,
  };
  return value * (scale[unit] ?? 1);
}

/**
 * Convert a budget's value (expressed in the field's display unit) into the
 * underlying raw unit used by [`observeField`]. Inverse of [`rawToDisplayValue`].
 */
export function displayToRawValue(field: BudgetField, displayValue: number): number {
  switch (field) {
    case 'duration_ns':
    case 'latency_p95_ns':
      return displayValue * 1_000_000; // ms → ns
    default:
      return displayValue;
  }
}

export function rawToDisplayValue(field: BudgetField, rawValue: number): number {
  switch (field) {
    case 'duration_ns':
    case 'latency_p95_ns':
      return rawValue / 1_000_000;
    default:
      return rawValue;
  }
}

export function formatObserved(field: BudgetField, raw: number): string {
  switch (field) {
    case 'duration_ns':
    case 'latency_p95_ns': {
      const ms = raw / 1_000_000;
      if (ms >= 1000) return `${(ms / 1000).toFixed(2)}s`;
      return `${ms.toFixed(1)}ms`;
    }
    case 'cost_usd':
      return `$${raw.toFixed(4)}`;
    default:
      return String(Math.round(raw));
  }
}

export function checkViolations(
  list: Budget[],
  summary: TraceSummary | null,
  totalCostUsd: number | null,
): BudgetViolation[] {
  if (!summary) return [];
  const out: BudgetViolation[] = [];
  for (const budget of list) {
    const observed = observeField(budget.field, summary, totalCostUsd);
    if (observed === null) {
      // Can't evaluate — skip silently.
      continue;
    }
    if (isViolated(budget.operator, observed, budget.value)) {
      out.push({ budget, observed });
    }
  }
  return out;
}
