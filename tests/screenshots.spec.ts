import { expect, test, type Browser, type BrowserContext, type Page } from '@playwright/test';
import path from 'node:path';

const BASE_URL = 'http://127.0.0.1:4173';
const SCREENSHOT_DIR = path.resolve('screenshots');

async function openPage(
	browser: Browser,
	route: string,
	theme: 'dark' | 'light' = 'dark'
): Promise<{ context: BrowserContext; page: Page }> {
	const context = await browser.newContext({
		baseURL: BASE_URL,
		colorScheme: theme,
		viewport: { width: 1388, height: 780 },
		reducedMotion: 'reduce'
	});

	await context.addInitScript((selectedTheme) => {
		localStorage.setItem('theme', selectedTheme);
		localStorage.setItem('checkUpdatesOnStartup', 'false');
	}, theme);

	const page = await context.newPage();
	await page.goto(route);
	await expect(page.locator('h1:visible')).toBeVisible();
	await page.evaluate(async () => {
		await document.fonts.ready;
		await Promise.all(
			Array.from(document.images, (image) => image.decode().catch(() => undefined))
		);
	});

	return { context, page };
}

async function capture(page: Page, filename: string) {
	await page.screenshot({
		path: path.join(SCREENSHOT_DIR, filename),
		animations: 'disabled'
	});
}

test('generate application screenshots', async ({ browser }) => {
	{
		const { context, page } = await openPage(browser, '/#/');
		await expect(page.getByRole('heading', { name: 'Welcome yobson!' })).toBeVisible();
		await expect(page.locator('.tiles img')).toHaveCount(10);
		await capture(page, 'home.png');
		await context.close();
	}

	{
		const { context, page } = await openPage(browser, '/#/', 'light');
		await expect(page.getByRole('heading', { name: 'Welcome yobson!' })).toBeVisible();
		await capture(page, 'light_home.png');
		await context.close();
	}

	{
		const { context, page } = await openPage(browser, '/#/about');
		await expect(page.getByRole('heading', { name: 'About' })).toBeVisible();
		await capture(page, 'about.png');
		await context.close();
	}

	{
		const { context, page } = await openPage(browser, '/#/settings');
		await expect(page.getByRole('heading', { name: 'Options' })).toBeVisible();
		await capture(page, 'settings.png');
		await context.close();
	}

	{
		const { context, page } = await openPage(browser, '/#/');
		await page.locator('.hamburger').click();
		await page.getByRole('button', { name: 'App ID' }).click();
		await expect(page.getByRole('heading', { name: 'Custom App ID' })).toBeVisible();
		await capture(page, 'appid.png');
		await context.close();
	}
});
