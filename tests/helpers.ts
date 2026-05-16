import { Page, expect } from '@playwright/test';

export const SAVE_KEY = 'emoji-niwa-save';

export type SaveData = {
  v: number;
  grid: number;
  items: Record<string, { emoji: string; size?: number }>;
  animals: Record<string, any>;
  history: any[];
  timeOfDay?: number;
  dayPaused?: boolean;
  weatherRandomOn?: boolean;
  lang?: string;
  lastStoreSize?: number;
  [k: string]: any;
};

// Attach console/page-error collectors. Returns a getter for assertions.
export function trackErrors(page: Page) {
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(`pageerror: ${e.message}`));
  page.on('console', (m) => {
    if (m.type() === 'error') errors.push(`console.error: ${m.text()}`);
  });
  return () => errors;
}

// Wait until the inline app script has fully booted.
async function waitReady(page: Page) {
  await page.waitForFunction(
    () => typeof (window as any).saveState === 'function' && !!document.getElementById('cv'),
  );
  // applyLang() runs at the end of boot and fills the weather button text.
  await expect(page.locator('#weather-btn')).not.toHaveText('');
}

// Load the app on a clean URL (no #w=/#r= share fragment) with empty storage.
export async function gotoFresh(page: Page) {
  await page.goto('/');
  await waitReady(page);
  await page.evaluate(() => {
    localStorage.clear();
    sessionStorage.clear();
  });
  await page.goto('/');
  await waitReady(page);
}

export async function readSave(page: Page): Promise<SaveData | null> {
  return page.evaluate((key) => {
    const raw = localStorage.getItem(key);
    return raw ? JSON.parse(raw) : null;
  }, SAVE_KEY);
}

export async function readSaveRaw(page: Page): Promise<string | null> {
  return page.evaluate((key) => localStorage.getItem(key), SAVE_KEY);
}

// 💾 Manual save is synchronous; returns the freshly persisted JSON.
export async function saveAndRead(page: Page): Promise<SaveData | null> {
  await page.click('#btn-save');
  return readSave(page);
}

function entityCount(s: SaveData | null): number {
  if (!s) return 0;
  return Object.keys(s.items || {}).length + Object.keys(s.animals || {}).length;
}

// Open the 🎲 generate popup and expand the "ランダム配置" accordion.
export async function openScatter(page: Page) {
  await page.click('#btn-gen');
  await expect(page.locator('#gen-popup')).toBeVisible();
  await page.click('.acc-header[data-acc="scatter"]');
  await expect(page.locator('#btn-scatter')).toBeVisible();
}

// Run a scatter, then poll manual-save until the staggered (setTimeout i*20)
// placement has settled, so assertions see the final world deterministically.
export async function scatterAndSettle(page: Page): Promise<SaveData> {
  await openScatter(page);
  // Keep the count small so the staggered fill finishes quickly.
  await page.locator('#sc-slider').evaluate((el: HTMLInputElement) => {
    el.value = '8';
    el.dispatchEvent(new Event('input', { bubbles: true }));
  });
  await page.click('#btn-scatter');
  await expect(page.locator('#gen-popup')).toBeHidden();

  let prev = -1;
  let stable: SaveData | null = null;
  for (let i = 0; i < 30; i++) {
    const s = await saveAndRead(page);
    const n = entityCount(s);
    if (n > 0 && n === prev) {
      stable = s;
      break;
    }
    prev = n;
    await page.waitForTimeout(120);
  }
  if (!stable) stable = await saveAndRead(page);
  expect(stable, 'scatter should produce a persisted world').not.toBeNull();
  return stable as SaveData;
}

// All scattered store tiles must be the single 3×3 🏪.
export function storeItems(s: SaveData) {
  return Object.entries(s.items || {}).filter(([, v]) => v.emoji === '🏪');
}
