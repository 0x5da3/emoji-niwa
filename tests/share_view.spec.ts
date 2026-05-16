import { test, expect } from '@playwright/test';
import {
  gotoFresh,
  readSave,
  readSaveRaw,
  scatterAndSettle,
  storeItems,
  trackErrors,
} from './helpers';

test.describe('share / viewing mode (#26–#28)', () => {
  test('viewing a shared world never clobbers autosave; 「これを自分のものに」claims it and dismisses the bar', async ({
    page,
  }) => {
    const errors = trackErrors(page);
    await gotoFresh(page);

    // Build & persist my own world.
    const mine = await scatterAndSettle(page);
    expect(storeItems(mine)).toHaveLength(1);
    const ownRaw = await readSaveRaw(page);
    expect(ownRaw).not.toBeNull();

    // Canonical share URL, exactly as #btn-share builds it (no clipboard dep).
    const shareUrl: string = await page.evaluate(
      async () =>
        location.origin +
        location.pathname +
        '#w=' +
        (await (window as any).encodeWorld((window as any).worldPayload())),
    );
    expect(shareUrl).toContain('#w=');

    // Full document load at the share URL → viewing mode.
    await page.goto('about:blank');
    await page.goto(shareUrl);
    await expect(page.locator('#visiting-bar')).toBeVisible();
    await expect(page.locator('#btn-visit-keep')).toBeVisible();

    // Autosave isolation: entering the shared world must not rewrite my save,
    // and an explicit 💾 while viewing is a no-op.
    expect(await readSaveRaw(page)).toBe(ownRaw);
    await page.click('#btn-save');
    await page.waitForTimeout(200);
    expect(await readSaveRaw(page)).toBe(ownRaw);

    // Claim it: bar must disappear (#28) and the shared world becomes mine.
    await page.click('#btn-visit-keep');
    await expect(page.locator('#visiting-bar')).toBeHidden();

    const claimed = await readSave(page);
    expect(claimed).not.toBeNull();
    expect(await readSaveRaw(page)).not.toBe(ownRaw);
    expect(storeItems(claimed!)).toHaveLength(1);
    expect(storeItems(claimed!)[0][1].size).toBe(3);
    expect(Object.keys(claimed!.items).length).toBeGreaterThan(0);

    expect(errors(), errors().join('\n')).toHaveLength(0);
  });

  test('「自分の箱庭に戻る」 leaves viewing mode and restores my own world', async ({
    page,
  }) => {
    await gotoFresh(page);
    const mine = await scatterAndSettle(page);
    const ownRaw = await readSaveRaw(page);

    const shareUrl: string = await page.evaluate(
      async () =>
        location.origin +
        location.pathname +
        '#w=' +
        (await (window as any).encodeWorld((window as any).worldPayload())),
    );
    await page.goto('about:blank');
    await page.goto(shareUrl);
    await expect(page.locator('#visiting-bar')).toBeVisible();

    await page.click('#btn-visit-back');
    await expect(page.locator('#visiting-bar')).toBeHidden();
    // Back to a no-hash URL and my untouched save.
    expect(new URL(page.url()).hash).toBe('');
    expect(await readSaveRaw(page)).toBe(ownRaw);
    expect(storeItems((await readSave(page))!)).toHaveLength(1);
    void mine;
  });

  test('#btn-share is wired up', async ({ page }) => {
    await gotoFresh(page);
    await scatterAndSettle(page);

    let dialogUrl = '';
    page.on('dialog', (d) => {
      dialogUrl = d.defaultValue() || d.message();
      d.dismiss();
    });

    await page.click('#btn-share');

    // Either clipboard got the URL, or the prompt fallback exposed it, or at
    // minimum a toast confirmed the action — any of these proves it's wired.
    let clip = '';
    try {
      clip = await page.evaluate(() => navigator.clipboard.readText());
    } catch {
      /* clipboard not permitted in this engine */
    }
    const url = clip || dialogUrl;
    if (url) {
      expect(url).toContain('#w=');
    } else {
      await expect(page.locator('#toast')).toBeVisible();
    }
  });
});
