import { beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';
import {
  BUDGET_FIELDS,
  budgets,
  checkViolations,
  displayToRawValue,
  fieldLabel,
  fieldUnit,
  formatObserved,
  rawToDisplayValue,
  type Budget,
} from './budgets';
import type { TraceSummary } from '../lib/types';

const summary = {
  span_count: 7,
  error_count: 2,
  llm_span_count: 4,
  total_duration_ns: 4_230_000_000,
  latency_p95_ns: 2_200_000_000,
} as unknown as TraceSummary;

function budget(over: Partial<Budget> = {}): Budget {
  return { id: 'b1', field: 'error_count', operator: '>', value: 0, ...over };
}

describe('budget field metadata', () => {
  it('labels and units every field it offers', () => {
    for (const field of BUDGET_FIELDS) {
      expect(fieldLabel(field.value)).toBe(field.label);
      expect(fieldUnit(field.value)).toBe(field.unit);
    }
  });

  it('falls back to the raw name for an unknown field', () => {
    expect(fieldLabel('nonsense' as never)).toBe('nonsense');
    expect(fieldUnit('nonsense' as never)).toBe('');
  });
});

describe('unit conversion', () => {
  it('shows durations in milliseconds and stores them in nanoseconds', () => {
    expect(displayToRawValue('duration_ns', 30)).toBe(30_000_000);
    expect(displayToRawValue('latency_p95_ns', 1)).toBe(1_000_000);
    expect(rawToDisplayValue('duration_ns', 30_000_000)).toBe(30);
    // Counts and dollars pass through untouched.
    expect(displayToRawValue('error_count', 3)).toBe(3);
    expect(rawToDisplayValue('cost_usd', 0.5)).toBe(0.5);
  });

  it('formats an observed value per field type', () => {
    expect(formatObserved('duration_ns', 4_230_000_000)).toBe('4.23s');
    expect(formatObserved('latency_p95_ns', 250_000_000)).toBe('250.0ms');
    expect(formatObserved('cost_usd', 0.0384)).toBe('$0.0384');
    expect(formatObserved('span_count', 7.4)).toBe('7');
  });
});

describe('checkViolations', () => {
  // A budget states the condition that must HOLD ("errors < 1"); a violation is
  // the condition failing, which is the opposite of how it first reads.
  it('reports nothing without a loaded trace', () => {
    expect(checkViolations([budget()], null, null)).toEqual([]);
  });

  it('flags a budget whose condition fails, with the observed value', () => {
    const found = checkViolations(
      [budget({ field: 'error_count', operator: '<', value: 1 })],
      summary,
      null,
    );
    expect(found).toHaveLength(1);
    expect(found[0].observed).toBe(2);
  });

  it('stays quiet while the condition holds', () => {
    expect(
      checkViolations([budget({ field: 'error_count', operator: '<', value: 5 })], summary, null),
    ).toEqual([]);
  });

  it('honours every operator', () => {
    // Observed error_count is 2.
    const cases: [Budget['operator'], number, boolean][] = [
      ['>', 1, false],
      ['>', 5, true],
      ['>=', 2, false],
      ['>=', 3, true],
      ['<', 5, false],
      ['<', 1, true],
      ['<=', 2, false],
      ['<=', 1, true],
      ['==', 2, false],
      ['==', 3, true],
    ];
    for (const [operator, value, violated] of cases) {
      const found = checkViolations([budget({ operator, value })], summary, null);
      expect(found.length === 1, `${operator} ${value}`).toBe(violated);
    }
  });

  it('skips a field it cannot observe rather than reporting a false breach', () => {
    // Cost is unknown until pricing resolves, and p95 only arrives as a
    // formatted string this summary does not carry.
    expect(
      checkViolations([budget({ field: 'cost_usd', operator: '<', value: 1 })], summary, null),
    ).toEqual([]);
    expect(
      checkViolations([budget({ field: 'cost_usd', operator: '<', value: 0.01 })], summary, 0.1),
    ).toHaveLength(1);
    expect(
      checkViolations([budget({ field: 'latency_p95_ns', operator: '<', value: 1 })], summary, null),
    ).toEqual([]);
  });

  it('reads p95 back out of its formatted display string', () => {
    const withDisplay = { ...summary, latency_p95_display: '750.0ms' } as unknown as TraceSummary;
    const found = checkViolations(
      [budget({ field: 'latency_p95_ns', operator: '<', value: 500_000_000 })],
      withDisplay,
      null,
    );
    expect(found).toHaveLength(1);
    expect(found[0].observed).toBe(750_000_000);
  });
});

describe('budgets store', () => {
  beforeEach(() => {
    localStorage.clear();
    budgets.clear();
  });

  it('adds, updates and removes, persisting each time', () => {
    const added = budgets.add('span_count', '<', 100);
    expect(get(budgets)).toHaveLength(1);
    expect(localStorage.getItem('widescope:budgets')).toContain('span_count');

    budgets.update(added.id, { value: 50 });
    expect(get(budgets)[0].value).toBe(50);

    budgets.remove(added.id);
    expect(get(budgets)).toEqual([]);
  });

  it('survives storage failures without losing the in-memory list', () => {
    const setItem = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('quota');
    });
    budgets.add('error_count', '>', 0);
    expect(get(budgets)).toHaveLength(1);
    setItem.mockRestore();
  });
});
