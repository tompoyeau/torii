# Changelog (English)

English translation of `CHANGELOG.md`, which remains the source. This file feeds the
English release notes page (`npm run build:notes` → `site/en/release-notes/`).
Add each new version here at the same time as in `CHANGELOG.md`, with the same
`## X.Y.Z` heading. UI labels quoted in a note must match the English app.

## 0.21.0
- **Torii speaks English.** The whole interface is translated, down to error messages, the menu of the icon next to the clock and the banner telling you a friend is playing. Pick the language in Settings → General: Français, English, or follow Windows.
- **If you were already using Torii, nothing changes for you**: it stays in French, with French prices. Only new installations start in English — the language the most people can read.
- **Game descriptions and genres follow the language you pick.** Torii asks Steam and GOG for them again in that language, and keeps what it already got in the other one: switching back downloads nothing. Game pages already on screen change on restart, and a button offers to do it right away.
- **A region, separate from the language.** It decides the prices in the Store and your wishlist: you can read Torii in English with prices in euros. Sixty-one countries are available, with the amounts and currency the price comparison actually uses for each — Switzerland and Sweden are in euros, Mexico in dollars. An unknown country falls back to US prices.
- **Every price shows in its own currency, its own way**: “44,99 €”, “$44.99”, “¥8,499” — the yen has no cents.
- **Instant Gaming only shows up when the other prices are in euros.** This seller only sells in euros: next to prices in dollars or pounds, its row wouldn't compare to anything. The comparison says so, instead of suggesting an offer is missing.
- **Price alerts no longer go haywire when you change region.** Switching currencies would have compared euros to dollars and announced fake price drops: tracking starts over, silently.
- **Game pages that were missing may come back.** Descriptions are now requested from Steam's widest catalog, no longer your country's, where some games are hidden.

## 0.20.3
- **Overwatch 2 gets its page back: cover, description, screenshots and genre.** The previous two versions had each ruled out a wrong answer without managing to find the right one. The reason: the database only knows this game as “Overwatch”, and its search never brings up that entry when asked for “Overwatch 2” — it only returns add-ons. Torii now explicitly asks for a game's other names, instead of hoping to come across them. Games left without a page for this reason catch up on their own the next time Torii starts.

## 0.20.2
- **No more pattern over covers.** The previous version had toned these white lines back down to their original subtlety; they were still visible to anyone who had noticed them. They're simply gone: a cover is a finished image, it doesn't need retouching.
- **Game pages that showed a DLC are finally fixed.** 0.20.1 did fix the search, but the wrong answer stayed stored on your disk — so Overwatch 2 kept showing the description of a coin bundle. Pages guessed from the title are now fetched again; Steam and GOG ones, which can't pick the wrong game, are kept as they are.

## 0.20.1
- **Covers are sharp again.** A white diagonal pattern had settled over every thumbnail in the library — a texture effect meant to stay invisible that, after an optimization, had started to show. It's imperceptible again, without losing the smoothness gained.
- **The featured game finally has its banner.** A game you had never opened showed on a plain gradient, while its page did show its image. Both now fetch the same thing.
- **Games that couldn't be found under their commercial name are recognized.** Overwatch 2 is the example: the database only knows this game as “Overwatch”, with “Overwatch 2” as an alternative name — so it stayed without a cover or description, forever. Alternative names are now taken into account, and missing pages catch up on their own the next time Torii starts.
- **A game page can no longer show a DLC's content.** For games from a launcher without its own catalog, Torii guessed the page by searching the title on the Steam Store — and could land on a coin bundle or a battle pass, taking its description, image and year. Overwatch 2 was the example. Expansions, bundles and add-on content are now ruled out, and the search looks for the exact name everywhere before settling for a close one.
- **A detected game can no longer take a DLC's name.** Same kind of problem in the automatic recognition of games from no launcher: “Overwatch 2: Invasion Bundle” looked enough like “Overwatch 2” to win.
- **Accents no longer break recognition.** “Hadès” and “Hades” were two different games for Torii, one letter apart.
- **On cards, hide and favorite have swapped places.** The star moves to the right: since it's the only one of the two buttons that stays visible when you're not hovering the card, it finally sits in the corner of the cover instead of being pushed away by an invisible button.

## 0.20.0
- **Game pages lost to a network drop come back.** When the connection failed at the wrong moment, Torii remembered “this game doesn't exist” instead of “I couldn't ask” — and never reopened the question: the game stayed without a description, genre or cover, permanently. This has been fixed for new games for a while, but pages already lost stayed lost. They're now looked up again, once, the next time Torii starts. Pages you already have aren't downloaded again: only the missing ones are.
- **A “Report a problem” button**, in Settings → About & maintenance. It opens a report already filled in with your Torii version and reminds you where to find the log. Until now, running into a bug and knowing where to report it were two different things.

## 0.19.0
- **A game bought and then launched from its launcher no longer shows up twice.** You buy a game on Steam and launch it right away: Torii hadn't seen it come in yet and filed it under “No launcher”. The next scan added the real Steam game next to it — the same game, twice, permanently. Torii now checks whether it's a freshly installed game before concluding, and duplicates already there disappear on their own at the next scan.
- **“Manual” and “No launcher” are now a single category.** Two sidebar entries for the same idea — a game no launcher provides — and two different labels on neighboring cards. There's only one now.
- **“In common” has left “Discover” to join your library**, under “Family”. It isn't about buying anything: this view compares your games with your friends'.
- **The Wishlist and “In common” finally have a search that works.** The one in the top bar filtered your library: on these two screens, which show something else, typing in it did nothing. Each one now has its own.
- **A cross to clear the top bar search in one click**, instead of selecting everything by hand.
- **You can see and sign out the devices connected to your Torii account** (Settings → Torii network). An open session used to be invisible: no way to know an old PC was still signed in, so no way to sign it out. A session unused for six months now ends on its own.
- **Torii can finally be used with the keyboard and is read out properly.** The app declared itself as English, which made a screen reader read the whole French interface with an English voice. Windows keep focus instead of letting Tab wander through the page behind them, and give it back to the original button when closing. Confirmation messages are announced. Input fields show they have focus again.
- **Light gray text is readable.** Dates, counters and subtitles fell below the recommended contrast threshold, in both themes. ⚠️ In the light theme, the accent color is a little deeper: white on top of it wasn't readable enough on action buttons.
- **Your friends still see you start a game, but Torii talks a lot less.** It sent its status every 30 seconds all the time, including at night when nobody is online. It now adapts its pace: 30 seconds as soon as someone is playing, a minute and a half when friends are around, five minutes when you're alone. In practice nothing changes, except that the service can hold five times as many people.
- **A game no longer loses its description for good because the network hiccuped.** A drop at the wrong moment was remembered as “this game doesn't exist”, and the game stayed without a description, genre or cover forever. Only a confirmed absence is remembered now.
- **Display fixes**: in Settings, the spacing between a setting's title and its explanation differed from one section to another, to the point where some descriptions touched their title.
- Under the hood, this is mostly groundwork for a wider release: the service that provides covers and descriptions can no longer be used by just anyone, the Torii network API is protected against abuse, and the app window enforces a strict security policy.

## 0.18.0
- **Every friend has their own page in Torii.** Clicking someone used to open their Steam page in your browser — a page that only knows about Steam, says nothing about their GOG or Epic games, and obviously has no “view their library” button. Everything Torii knows about a person now lives in one place: their avatar, what they're playing, which channel they're connected through, the games you have in common and a preview of their library.
- **Even Steam friends have a page**, and it's deliberately almost empty: it explains what Torii can't know about them, and offers your friend code to copy right there to fix that.
- **The friends list is lighter**: the “library” and “remove” buttons are gone from each row. Everything about a person happens on their page; the list just leads to it. Removing a friend now lives behind the gear on their profile, with a real confirmation — it's the only action in the app that can't be undone.
- **The Steam profile opens in a Torii window**, no longer in your browser: going outside to look for information you came looking for inside meant losing the app along the way.
- **Steam Family Sharing games no longer count as owned games.** A game borrowed from family showed up in “In common” as if your friend owned it: no way to tell who owned what, or whether you could play it together. “In common” now means what it says — you both own it. These games stay visible in your friend's library, where they're tagged “Steam family”.
- **In a friend's library, clicking a game you don't have finally opens something**: its price comparison, on top of the page. Until now the click did nothing at all — even though browsing someone's library is precisely about finding what you don't have.
- **No more star or crossed-out eye on other people's games.** These buttons act on YOUR library; offering them on someone else's covers suggested otherwise. Same for the right-click menu, which offered “Uninstall” on a friend's game.
- **Text is in French.** Descriptions and genres of Steam and GOG games, categories, and Instant Gaming links point to the French site. Descriptions of games that aren't on Steam or GOG stay in English: the database that provides them only exists in that language.
- **Display fixes**: filter buttons showed as raw Windows buttons in a friend's library, in “In common” and in the Wishlist; covers there took the width of their text instead of a uniform size; and a library preview now follows the cover size chosen in settings.

## 0.17.0
- **Your friends finally see what you own, not just what you play.** Your library can be stored on your Torii account: your friends browse it from the Friends view, with your Steam, GOG, Epic, Ubisoft and manually added games together in a single list. A filter shows what you have that they don't, or what you both have.
- **Two settings, not one.** *Sync* stores your library so you get it back on your other devices; *share* also lets your friends see it. One works without the other, and both are **off by default**: after this update, nothing leaves your PC until you ask for it. Everything turns off in one click, and turning it off deletes what was sent.
- **Your hidden games stay hidden.** They and games marked “don't share” are never sent, even with sharing on.
- **“In common” is no longer limited to Steam.** The view now cross-checks both sources: a friend who isn't on Steam can show up there, and a GOG, Epic or manually installed game can finally be “in common”. Each friend is counted only once.
- **The Friends view shows how each person is connected**: Steam, Torii, or both. A dimmed badge flags a friend on that side who isn't connected there right now — useful to know when Torii can't see what they're playing.
- Under the hood, these libraries are what the mobile app will rely on to know yours.

## 0.16.1
- **Minecraft is finally detected.** It runs in a Java virtual machine, which Torii set aside out of caution: the file path only mentions the engine, never the game. Torii now asks Windows for its name, which Windows knows. Same for other games launched through a shared engine.
- **A brand-new game is seen from its very first session.** Windows only adds a game to its list when it notices it, sometimes a few seconds after launch — Torii concluded too quickly and waited for the next session. It now gives an unknown game two minutes.
- **No more double notifications for your friends.** A freshly detected game is only announced once its real title is known, instead of going out under its temporary name and then being corrected — which counted as two launches.

## 0.16.0
- **Your friends finally see games that come from no launcher.** Genshin Impact, Dofus, a Game Pass game, a manually installed game: until now Torii didn't see them, so nobody knew you were playing them. They're now spotted on their own, added to your library under “No launcher”, with their real title and cover — and they count in “Recently played” like the rest.
- Torii doesn't guess at random: it relies on the list of games **Windows already keeps** for its Game Bar. So a browser or a word processor won't be announced to your friends. If a title still comes out wrong, fix it from the game's page, and “Remove from library” sets it aside for good.
- **New startup screen**: the gate builds itself — the pillars rise, the crossbeams open, the lintel settles.
- **Couch mode has been removed.** Nobody used it, and it dragged its own navigation, screens and settings along. Torii now has a single interface, and the “Startup mode” setting is gone from Settings.

## 0.15.0
- **Account creation redesigned**: everything happens in a single window — email address, code received by email, then display name. The display name is no longer a suggestion you could skip: your account is created with the one you choose, not with a name derived from your email address.
- **Nothing is created until you're done.** If you close Torii in the middle of signing up, no account is saved and your address stays free.
- **Delete your Torii account**, from Settings → Torii network. Display name, friend code and connections are removed from the server; your library and settings stay on your computer. You have to type your display name to confirm — it's permanent.
- **Remove a friend**: a trash icon appears when hovering each Torii friend in the Friends view, with a confirmation right there. Removal applies on both sides. Steam friends are managed from Steam.
- **A Steam account can no longer be linked to two Torii accounts.** It created duplicates for your friends — one of them an inert row — and let someone use another person's Steam avatar.
- Scrollbars in the app's colors, in both light and dark themes.
- Friends view: button aligned at the bottom of cards, and “right now” for everyone.

## 0.14.0
- **A friend starts a game, you see it**: a banner shows for a few seconds at the top right of the screen, above other windows, without ever stealing your keyboard. Can be turned off in Settings → Torii network. A game in exclusive fullscreen may hide it.
- **Diagnostic log**: Torii now writes what happens to it in a file (Settings → About → “Open log”). If the app closes on its own or a screen stays blank, this file says why. Three crashes hit by players had left no trace until now.
- **Your Steam account and your Torii account find each other on their own**: as soon as both are connected, your Steam ID is added to your Torii account and your Steam friends already on Torii can find you. It only happens once: if you turn this visibility off, it stays off.
- Friends view: the “Play” button is aligned at the bottom of each card, and every friend in a game shows “right now” — the duration was only known for some of them, which looked like missing information.

## 0.13.0
- **Friends view redesigned**: people who are playing come first, on cards showing the game large and for how long. Offline friends are collapsed, available ones fit on a single line.
- **“You have it too”**: when a friend is playing a game you own, their card tells you and offers to launch it — even if you don't have it on the same launcher. That's something Torii does that nobody else does.
- **Choose what your friends see**: game visible, online only, or invisible. In “online” mode, nobody knows what you're playing, even mid-game.
- **A display name**: chosen when signing up and editable in settings. Before, your name was derived from your email address without you asking for it.
- The account button at the top right now opens your Torii account settings instead of your Steam profile in the browser.
- **“Don't share this game”** is now also in the gear menu on a game's page, not just in the right-click menu.
- Windows' context menu (“Inspect”, “Save as”…) no longer opens in the app. It's still available in input fields, so you can paste.

## 0.12.2
- **Fix**: the “Visible to my Steam friends” button stopped responding once turned off. It's reliable again, both ways.
- **Fix**: unlinking your Steam account from Torii didn't work — the ID stayed stored on the server while the app showed otherwise.
- The button becomes clickable as soon as you connect Steam, without restarting Torii.

## 0.12.1
- **Major fix**: Torii closed on its own right after signing in to Steam (or GOG / Epic). The sign-in window, when closing, was mistaken for the main window and made the app quit. It affected every new installation.
- **Finding your Steam friends** finally works: the search existed on the server but no screen called it. It's now in Settings → Torii network, and you can add the people found directly.
- The “visible to my Steam friends” option is disabled, with an explanation, as long as no Steam account is connected — it had no effect in that case.
- **Torii now stays running in the background** when you close the window, to keep detecting your sessions. This can be turned off in Settings, and “Quit” stays available by right-clicking the icon next to the clock.

## 0.12.0
- **Torii network**: create an account and see what your friends are playing **whatever their launcher** — Steam, Epic, GOG, Riot, a manually added game… Something neither Steam nor Discord can do.
- **Passwordless sign-in**: you enter your address, you get a 6-digit code, and that's it. Nothing to remember, nothing to lose.
- **Adding a friend** uses a short code you give them directly. Nobody can find you by guessing your email address.
- **Sharing is off by default.** Until you turn it on, absolutely nothing about what you play leaves your PC. A “Visible / Invisible” badge lets you disappear in one click, and right-clicking a game is enough to never share it.
- **A single friends list**, fed by Steam and by Torii: the same person only appears once, with their Steam avatar and the game seen by Torii. Neither source replaces the other — Steam knows who's online even when Torii is closed, Torii sees games from every launcher.
- **No history** is kept: your status expires after 90 seconds and is never archived.

## 0.11.0
- **“Recently played” is finally right**: Torii spots sessions launched **from anywhere** — Steam, the desktop, a shortcut — not just those started from Torii. A game moves to the top of the list within seconds, dated from the actual time it started (not the moment Torii noticed it).
- **Lighter while you play**: session tracking now uses about 140 times less than before. In practice, it's invisible to in-game performance.
- A game already running when Torii starts is dated correctly, instead of jumping to the top every time the app opens.

## 0.10.0
- **Adding to the Steam wishlist fixed**: adding a game from the Store finally pushes it to your real Steam wishlist. The API Torii used had been retired by Valve and silently refused every addition; the confirmation message now says whether Steam followed.
- **Missing covers in the wishlist**: more than half of the games (new releases, unreleased games) have no cover on the Steam CDN and showed as a gradient. Torii now falls back to a backup cover.
- **Consistent display**: wishlist games use the same card as the library and the Store (same size, same overlaid title, same hover).
- **Hidden sellers respected everywhere**: a hidden seller was only left out on a game's page — it came back as the best price in the storefront, the search and the wishlist. Fixed.
- **Instant startup**: your library shows immediately at launch, without waiting for accounts to sync, then updates on its own.
- **Protected credentials**: sign-in tokens for your accounts (Steam, GOG, Epic) are now encrypted on disk by Windows. The conversion is automatic on first launch, and your accounts stay connected.
- **Add a game by hand, without typing a path**: “Browse…” opens Windows Explorer to pick the executable, the folder and the cover (an image from your disk works). The title and folder are guessed from the chosen file.
- **Edit a manually added game**: title, executable, folder and cover can be fixed afterwards, from its page or by right-clicking. Favorite and hidden status are kept.
- **Fix**: on a manually added game's page, the “Uninstall” button did nothing. It becomes “Remove from library” and works.
- **Back from the price comparison**: checking a library game's prices and then going back takes you to the game's page, instead of leaving you on the Store's storefront.
- **Display fix**: the library scrollbar stayed visible — and usable — behind a game's page or Settings.
- **Price alerts**: fixed tracking that could mix up several games added from the Store.

## 0.9.9
- **Universal wishlist**: add any game to your wishlist from the Store (♥ button on the page and on cards). If the game exists on Steam, it's also added to your Steam wishlist. A short message confirms it.
- **Price alerts**: they now track your whole wishlist (Steam + games added in Torii).
- **Instant Gaming**: out-of-stock offers are flagged (“Out of stock”) and are no longer offered as the best price.
- **Startup screen** while the library loads.
- **Notification area fixes**: no more “ghost” instance stuck in the tray, relaunching properly reopens the existing window, and startup no longer makes the window flicker.
- Removed the “Recent” category and the grid/list button (the default layout is set in Settings).

## 0.9.8
- **Fix**: when Torii keeps running in the background (“close to the notification area” option), relaunching it now brings the existing window to the front instead of opening a second instance that prevented it from opening.

## 0.9.7
- **Settings redesigned**: a real settings window with navigation by category (General, Hidden games, Hidden sellers, Accounts & launchers, About). Launcher connection management is built in.
- **Launch Torii when Windows starts** (option).
- **Notification area**: Torii can start minimized and/or minimize to the notification area when closed instead of quitting.
- **Come back when a game closes** (option): Torii minimizes during the session and reopens the game's page when you close it.
- **New settings**: startup mode (Desktop/Couch), default filter/sort/layout, library density, theme (light/dark/system), reduce motion.
- **About & maintenance**: version number, manual update check, button to clear the cache.
- **Store**: you can hide the sellers of your choice from the price comparison (list managed from Settings), and scroll arrows on the screenshot gallery.
- **Navigation**: the mouse “back” button goes back in the app.
- **Battle.net**: “Uninstall” now opens the client on the game's page instead of File Explorer.
- Removed the “Recent” category from the sidebar.

## 0.9.6
- **Battle.net**: “Play” and “Install” now work even when the Battle.net client is already open (before, nothing happened). “Play” launches the game directly.

## 0.9.5
- **Install a game from Torii**: for a game you own but haven't installed, the button becomes “Install” and starts the installation through the launcher (without counting it as a session played).
- **Fixed Epic launching/installing** of games that aren't installed (e.g. Just Cause 4).
- **Detection of installed EA games** (the EA app), for real “Play / Install” like other launchers.
- **Store**: a “Surprise me” button to show a random selection of games.
- **Game page**: arrows to scroll through the screenshot gallery.

## 0.9.4
- **Steam achievements on a game's page**: your real achievements (icon, description, unlock date) with actual progress and a collapsible preview.
- **Playing now**: the live number of players on Steam games, in the page's stats.
- **Real launcher logos**: Epic, Ubisoft, Battle.net and Riot now show their official logo.
- **Game page**: options button back to the right size, new “Open file location” shortcut, and the Play / Options menus no longer stay open at the same time.
- **Store**: a “Surprise me” button to pick a selection of games to discover.

## 0.9.3
- **Sidebar redesigned**: cleaner navigation, official launcher icons, and a much more readable selected item.
- **Friends** moved to the top bar, with a badge showing how many friends are online.
- New right-click **“Open file location”** on an installed game, to open its folder in File Explorer.
- Fix: some **uninstalled Epic games still showed as installed** (e.g. Palia) — they're now detected correctly.
- The **profile icon** at the top now shows your real Steam identity (name and avatar).
- Interface cleanup: the fake storage bar is gone; the button at the bottom of the sidebar now says **“Connect a launcher”** or **“Manage connections”** depending on your accounts.
- Fewer **antivirus false positives** (Windows Defender).

## 0.9.2
- New **Wishlist** section: your Steam wishlist with, for each game, the current best price, the discount and the all-time low — to spot at a glance what's on sale.
- Fixed the **Steam friends list** staying empty for accounts with a custom URL.
- Store: **navigation arrows** (and keyboard) in the screenshot viewer.
- New **“View in store”** button on a game's page, to compare its prices in one click.

## 0.9.1
- Fixed the **empty Steam friends list** for some users: the community session is now regenerated automatically (like the library), and signing in to Steam generates a clean cookie even when an old session from a previous version is present. If your friends list wasn't showing, reconnect your Steam account once.

## 0.9.0
- New **In common** section: find all the games you share with your Steam friends, with a multi-friend filter to see what you all own and play together.
- On every game page: **friends who also own the game** (clickable to their Steam profile) and, for Family Sharing games, the **number of copies available in the family**.
- Fixed **uninstalling Epic games**: Epic now opens directly on the right game.
- New, more recognizable icons for Riot, Ubisoft Connect and Battle.net.

## 0.8.0
- New **Store**: discover games to buy and compare prices across every PC store (Steam, GOG, Epic, Humble, Fanatical…) in euros, with the all-time low and a direct buy link. Search with instant suggestions. Instant Gaming prices are also shown on the product page.
- New **Friends** panel: find your Steam friends in one place, see who's online and what they're playing, with live refresh.

## 0.7.0
- Last session recorded when launching from Torii: games without stats (Riot, EA, Battle.net…) now show their last played date and move up in “Recently played”.

## 0.6.1
- Game page: fixed the stats panel going off-screen when there were screenshots, and a wider “About” area.

## 0.6.0
- Metadata enriched through IGDB for every game: description, screenshots, studio, year and backup cover — including games outside Steam (Fortnite, Valorant, Battle.net…) that had nothing until now.

## 0.5.0
- Genres through IGDB: the category filter now works for the whole library, including games outside Steam (Fortnite, Valorant, WoW…).

## 0.4.0
- Update banner redesigned (display fixed: full-width buttons, readable notes).
- Richer release notes (a real changelog per version).

## 0.3.0
- Category filter: filter games by genre, combinable with platforms and search.

## 0.2.0
- Context menu (right-click) on games: play, favorite, hide, uninstall.
- Add a game manually (title, executable, cover).
- Automatic app updates.
