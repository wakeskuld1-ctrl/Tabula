console.log("[verify_sessions_tabs] boot");
const { default: puppeteer } = await import("puppeteer");

// ### Change Log
// - 2026-03-14: Reason=Add UI-only session tab assertions; Purpose=cover default/read-only labels (TDD)
// - 2026-03-14: Reason=Standardize failure output; Purpose=improve debugging

const PORT = process.env.PORT || process.env.VITE_DEV_SERVER_PORT || "5174";
const BASE_URL = process.env.FRONTEND_URL || `http://127.0.0.1:${PORT}`;

const EXPECT_TABLIST_LABEL = "会话标签列表";
const EXPECT_ADD_TITLE = "新增沙盘";
const EXPECT_DEFAULT_LABEL = "默认/只读";
const EXPECT_READONLY_TAG = "只读";

// ### Change Log
// - 2026-03-14: Reason=Provide explicit failure exit; Purpose=fast fail in automation
const fail = (message) => {
  console.error(`CRITICAL: ${message}`);
  process.exit(1);
};

const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

(async () => {
  console.log("[verify_sessions_tabs] start");
  const globalTimeout = setTimeout(() => {
    fail("global timeout exceeded");
  }, 50000);

  let browser;
  try {
    browser = await puppeteer.launch({
      headless: true,
      args: ["--no-sandbox", "--disable-setuid-sandbox"]
    });
  } catch (e) {
    fail(`puppeteer launch failed: ${e?.message || String(e)}`);
  }

  const page = await browser.newPage();
  // ### Change Log
  // - 2026-03-14: Reason=Vite keeps HMR connections; Purpose=avoid networkidle0 timeout
  page.setDefaultTimeout(60000);
  page.on("console", (msg) => {
    const text = msg.text();
    if (!text.includes("HMR")) {
      console.log(`[Browser Console] ${msg.type().toUpperCase()}: ${text}`);
    }
  });
  page.on("response", (resp) => {
    const status = resp.status();
    if (status >= 400) {
      console.log(`[Browser Response] ${status} ${resp.url()}`);
    }
  });

  try {
    // ### Change Log
    // - 2026-03-14: Reason=HMR/WebSocket blocks networkidle0; Purpose=use domcontentloaded
    await page.goto(BASE_URL, { waitUntil: "domcontentloaded" });
    // ### Change Log
    // - 2026-03-14: Reason=Wait for table select; Purpose=ensure options are rendered
    await page.waitForSelector("select", { timeout: 20000 });
    console.log("[verify_sessions_tabs] table select ready");

    // ### Change Log
    // - 2026-03-14: Reason=App auto-selects first table; Purpose=skip manual select to avoid hangs
    await wait(4000);
    console.log("[verify_sessions_tabs] table auto-select assumed by app");

    // ### Change Log
    // - 2026-03-14: Reason=Wait for sessions to render; Purpose=ensure tabs ready
    await wait(1500);
    console.log("[verify_sessions_tabs] wait for sessions render");
    await page.waitForSelector(".sheet-bar", { timeout: 20000 });
    await page.waitForSelector(".sheet-tabs", { timeout: 20000 });
    await page.waitForSelector(".sheet-tab", { timeout: 20000 });

    const labelSnapshot = await page.evaluate(() => {
      const tabList = document.querySelector(".sheet-tabs");
      const addButton = document.querySelector(".sheet-add");
      const defaultTag = document.querySelector(".sheet-default-tag");
      const activeTab = document.querySelector(".sheet-tab.active");
      return {
        tabListLabel: tabList?.getAttribute("aria-label") || "",
        addTitle: addButton?.getAttribute("title") || "",
        defaultTagText: defaultTag?.textContent?.trim() || "",
        activeTabText: activeTab?.textContent?.trim() || ""
      };
    });

    if (labelSnapshot.tabListLabel !== EXPECT_TABLIST_LABEL) {
      fail(`tablist aria-label mismatch: ${labelSnapshot.tabListLabel}`);
    }
    if (labelSnapshot.addTitle !== EXPECT_ADD_TITLE) {
      fail(`add button title mismatch: ${labelSnapshot.addTitle}`);
    }
    if (labelSnapshot.defaultTagText !== EXPECT_READONLY_TAG) {
      fail(`default tag text mismatch: ${labelSnapshot.defaultTagText}`);
    }
    if (!labelSnapshot.activeTabText.includes(EXPECT_DEFAULT_LABEL)) {
      fail(`default tab label mismatch: ${labelSnapshot.activeTabText}`);
    }

    // ### Change Log
    // - 2026-03-14: Reason=Validate add session UI; Purpose=ensure new tab appears and is active
    const initialCount = await page.evaluate(() => document.querySelectorAll(".sheet-tab").length);
    await page.click(".sheet-add");
    await page.waitForFunction((count) => document.querySelectorAll(".sheet-tab").length > count, {}, initialCount);

    const afterAddSnapshot = await page.evaluate(() => {
      const tabs = Array.from(document.querySelectorAll(".sheet-tab"));
      const active = document.querySelector(".sheet-tab.active");
      return {
        count: tabs.length,
        activeText: active?.textContent?.trim() || ""
      };
    });

    if (afterAddSnapshot.count <= initialCount) {
      fail("add session did not create a new tab");
    }
    if (afterAddSnapshot.activeText.includes(EXPECT_DEFAULT_LABEL)) {
      fail("new session should become active after creation");
    }

    // ### Change Log
    // - 2026-03-14: Reason=Validate switching back to default; Purpose=confirm active tab changes
    const switchBack = await page.evaluate(() => {
      const defaultTag = document.querySelector(".sheet-default-tag");
      const defaultTab = defaultTag ? defaultTag.closest(".sheet-tab") : null;
      if (!defaultTab) return { ok: false, error: "default tab not found" };
      defaultTab.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      return { ok: true };
    });

    if (!switchBack.ok) {
      fail(switchBack.error);
    }

    const switchedSnapshot = await page.evaluate(() => {
      const active = document.querySelector(".sheet-tab.active");
      return active?.textContent?.trim() || "";
    });

    if (!switchedSnapshot.includes(EXPECT_DEFAULT_LABEL)) {
      fail("switch back to default tab failed");
    }

    console.log("[verify_sessions_tabs] success");
    clearTimeout(globalTimeout);
    await browser.close();
    process.exit(0);
  } catch (e) {
    if (browser) {
      await browser.close();
    }
    clearTimeout(globalTimeout);
    fail(e?.message || String(e));
  }
})();
