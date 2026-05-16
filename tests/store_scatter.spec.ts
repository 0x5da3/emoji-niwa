import { test, expect } from '@playwright/test';
import { gotoFresh, scatterAndSettle, storeItems, trackErrors } from './helpers';

test.describe('store / scatter', () => {
  test('scatter places exactly one 3×3 🏪 with a clear footprint', async ({ page }) => {
    const errors = trackErrors(page);
    await gotoFresh(page);

    const save = await scatterAndSettle(page);
    const grid = save.grid;
    const stores = storeItems(save);

    // Exactly one 🏪, and it is the 3×3 variant.
    expect(stores).toHaveLength(1);
    const [storeKey, storeVal] = stores[0];
    expect(storeVal.size).toBe(3);

    // No other item sits inside the store's 3×3 footprint.
    const [sc, sr] = storeKey.split(',').map(Number);
    for (let dc = 0; dc < 3; dc++) {
      for (let dr = 0; dr < 3; dr++) {
        const k = `${sc + dc},${sr + dr}`;
        if (k === storeKey) continue;
        expect(save.items[k], `footprint cell ${k} must be empty`).toBeUndefined();
      }
    }

    // Always leave at least one empty cell (count cap).
    const total = Object.keys(save.items).length + Object.keys(save.animals).length;
    expect(total).toBeLessThanOrEqual(grid * grid - 1);
    expect(total).toBeGreaterThan(0);

    expect(errors(), errors().join('\n')).toHaveLength(0);
  });

  test('re-scattering keeps the single-store invariant', async ({ page }) => {
    await gotoFresh(page);
    await scatterAndSettle(page);
    const save = await scatterAndSettle(page);

    const stores = storeItems(save);
    expect(stores).toHaveLength(1);
    expect(stores[0][1].size).toBe(3);
  });
});
