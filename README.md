# rbxsync

Declaratively manage Roblox game passes, badges, and developer products from a single TOML config file.

rbxsync syncs your local configuration to Roblox, tracks remote state in a lockfile, detects icon changes with [BLAKE3](https://github.com/BLAKE3-team/BLAKE3) hashing, and generates a Luau module with all your asset IDs.

## Features

- **Declarative config** - Define all your passes, badges, and products in a single `rbxsync.toml`
- **Two-way sync** - Push local changes to Roblox or pull remote state to your config and lockfile
- **Icon management** - Upload icons, detect changes via BLAKE3 hashing, download remote icons
- **Conflict detection** - Detects when remote icons differ from local and lets you choose which to keep
- **Code generation** - Generates a Luau module (+ optional TypeScript definitions) mapping resource names to asset IDs
- **Flat & nested styles** - Choose between flat path-like keys or nested tables
- **Custom codegen paths** - Remap sections and individual items to custom paths
- **Extra entries** - Inject pre-existing asset IDs into the generated file without syncing them
- **Alpha bleed** - Fixes resize artifacts on icons before uploading (enabled by default)
- **Duplicate detection** - Warns when multiple remote resources share the same name

## Installation

### Rokit

```sh
rokit add dev-bap/rbxsync
```

### Cargo

```sh
cargo install rbxsync
```

## Quick Start

### From scratch

```sh
rbxsync init
```

This creates a `rbxsync.toml` template. Edit it with your universe ID, creator info, and resources, then run:

```sh
rbxsync sync --api-key YOUR_API_KEY
```

### From existing remote resources

```sh
rbxsync init --from-remote --universe-id 123456 --api-key YOUR_API_KEY
```

This fetches all your existing passes, badges, and products, downloads their icons, and generates both the config and lockfile.

## Commands

### `rbxsync init`

Initialize a new config file.

| Flag | Description |
| --- | --- |
| `--from-remote` | Populate config from existing remote resources |
| `--universe-id` | Universe ID (required with `--from-remote`) |

### `rbxsync sync`

Sync local config to Roblox. Creates, updates, and tracks resources.

| Flag | Description |
| --- | --- |
| `--dry-run` | Show what would change without applying |
| `--only` | Only sync specific types: `passes`, `badges`, `products` (comma-separated) |
| `--badge-cost` | Expected cost in Robux when creating a badge (default: `0`) |

### `rbxsync pull`

Pull remote state into the config and lockfile. Remote is the source of truth: remote-visible fields (name, price, description, etc.) are updated in the config while config-only fields (icon, path, regional_pricing) are preserved. New remote resources are added to the config. Detects icon conflicts.

| Flag | Description |
| --- | --- |
| `--dry-run` | Show what remote state differs without writing anything |
| `--accept-remote` | Download remote icons and update local files |
| `--accept-local` | Keep local icons and re-upload on next sync |

### `rbxsync check`

Validate config, check lockfile consistency, and report if anything is out of sync.

### `rbxsync rename <resource> <old_key> <new_key>`

Rename a resource key in both config and lockfile. The display name is preserved automatically.

```sh
rbxsync rename passes VIP vip_pass
```

### `rbxsync list <resource>`

List remote resources. `resource` is one of: `passes`, `badges`, `products`.

## Configuration

rbxsync requires a `rbxsync.toml` file in the working directory (or specify with `--config`).

```toml
[experience]
universe_id = 123456789

[experience.creator]
type = "group"         # "user" or "group"
id = 35757120

[codegen]
output = "src/shared/GameIds.luau"
# typescript = false   # Also generate a .d.ts file
# style = "flat"       # "flat" (default) or "nested"

[icons]
bleed = true           # Apply alpha bleed before uploading (default: true)
dir = "icons"          # Directory for downloaded icons (default: "icons")

[passes.VIP]
price = 499
description = "VIP access to exclusive areas"
icon = "icons/vip.png"

[passes.Premium]
price = 999
description = "Premium membership"
icon = "icons/premium.png"

[badges.Welcome]
description = "Welcome to the game!"
icon = "icons/welcome.png"
enabled = true

[products.Coins100]
price = 99
description = "100 coins"
icon = "icons/coins.png"
```

### Experience

| Field | Type | Description |
| --- | --- | --- |
| `universe_id` | `u64` | Your Roblox universe ID |
| `creator.type` | `string` | `"user"` or `"group"` |
| `creator.id` | `u64` | Your Roblox user or group ID |

### Codegen

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `output` | `string` | -- | Path to generate the Luau module (omit to disable) |
| `typescript` | `bool` | `false` | Also generate a TypeScript definition file (`.d.ts`) |
| `style` | `string` | `"flat"` | `"flat"` or `"nested"` (see [Code Generation](#code-generation)) |

#### `[codegen.paths]`

Override the default section name for each resource type. Dot-separated segments become either a prefix (flat) or nested tables (nested).

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `passes` | `string` | `"passes"` | Path for game passes |
| `badges` | `string` | `"badges"` | Path for badges |
| `products` | `string` | `"products"` | Path for developer products |

```toml
[codegen.paths]
passes = "player.vips"
badges = "rewards"
products = "shop.items"
```

#### `[codegen.extra]`

Inject pre-existing asset IDs into the generated file. Useful for assets that were created outside rbxsync (e.g. legacy passes, manually created products) but that you still want available in code.

```toml
[codegen.extra]
"passes.legacy_vip" = 1234567
"products.starter_pack" = 9876543
```

### Icons

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `bleed` | `bool` | `true` | Apply alpha bleed to fix resize artifacts |
| `dir` | `string` | `"icons"` | Directory for icons downloaded by `pull --accept-remote` |

### Game Passes

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `name` | `string` | No | Display name (defaults to the TOML key) |
| `price` | `u64` | No | Price in Robux (omit for free) |
| `description` | `string` | No | Pass description |
| `icon` | `string` | No | Path to icon file |
| `for_sale` | `bool` | No | Whether the pass is for sale (default: `true`) |
| `regional_pricing` | `bool` | No | Enable regional pricing (default: `false`) |
| `path` | `string` | No | Override the codegen path for this item |

### Badges

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `name` | `string` | No | Display name (defaults to the TOML key) |
| `description` | `string` | No | Badge description |
| `icon` | `string` | No | Path to icon file |
| `enabled` | `bool` | No | Whether the badge is active (default: `true`) |
| `path` | `string` | No | Override the codegen path for this item |

### Developer Products

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `name` | `string` | No | Display name (defaults to the TOML key) |
| `price` | `u64` | **Yes** | Price in Robux |
| `description` | `string` | No | Product description |
| `icon` | `string` | No | Path to icon file |
| `for_sale` | `bool` | No | Whether the product is for sale (default: `true`) |
| `regional_pricing` | `bool` | No | Enable regional pricing (default: `false`) |
| `store_page` | `bool` | No | Show on the store page (default: `false`) |
| `path` | `string` | No | Override the codegen path for this item |

## Authentication

rbxsync uses the [Roblox Open Cloud API](https://create.roblox.com/docs/cloud/open-cloud). Create an API key at https://create.roblox.com/dashboard/credentials and pass it via `--api-key`:

```sh
rbxsync sync --api-key YOUR_API_KEY
```

Or read it from a file:

```sh
rbxsync sync --api-key $(cat apikey.txt)
```

### Required API scopes

| Resource | Scopes | Documentation |
| --- | --- | --- |
| Game Passes | `game-pass:read`, `game-pass:write` | [Game Passes API](https://create.roblox.com/docs/cloud/api/game-passes) |
| Developer Products | `developer-product:read`, `developer-product:write` | [Developer Products API](https://create.roblox.com/docs/cloud/api/developer-products) |
| Badges | `legacy-universe.badge:read`, `legacy-universe.badge:write`, `legacy-universe.badge:manage-and-spend-robux` | [Badges API](https://create.roblox.com/docs/cloud/api/badges), [Universes - Badges](https://create.roblox.com/docs/cloud/features/universes#badges) |
| Assets (icons) | `legacy-asset:manage` | [Assets](https://create.roblox.com/docs/cloud/features/assets#/) |

## Code Generation

When `codegen.output` is set, rbxsync generates a Luau module after every `sync`. The variable name is derived from the filename.

### Styles

#### Flat (default)

Items are stored with dot-separated path keys. This works well with TypeScript as the string-literal keys provide full autocomplete.

```toml
[codegen]
output = "src/shared/GameIds.luau"
style = "flat"
```

```lua
-- This file is auto-generated by rbxsync. Do not edit manually.

local GameIds = {
	["badges.Welcome"] = 98765,
	["passes.Premium"] = 67891,
	["passes.VIP"] = 67890,
	["products.Coins100"] = 11111,
}

return GameIds
```

Access: `GameIds["passes.VIP"]`

#### Nested

Items are organized into nested tables. Better for Luau since it provides direct table access without string keys.

```toml
[codegen]
output = "src/shared/GameIds.luau"
style = "nested"
```

```lua
-- This file is auto-generated by rbxsync. Do not edit manually.

local GameIds = {
	badges = {
		Welcome = 98765,
	},
	passes = {
		Premium = 67891,
		VIP = 67890,
	},
	products = {
		Coins100 = 11111,
	},
}

return GameIds
```

Access: `GameIds.passes.VIP`

### Custom paths

Use `[codegen.paths]` to remap entire sections, or per-item `path` to override individual items:

```toml
[codegen.paths]
passes = "player.vips"
badges = "rewards"
products = "shop.items"

[products.special_offer]
price = 99
path = "shop.specials"       # overrides the section default
```

With `style = "nested"`, this produces:

```lua
local GameIds = {
	player = {
		vips = {
			VIP = 67890,
		},
	},
	shop = {
		items = {
			Coins100 = 11111,
		},
		specials = {
			special_offer = 12345,
		},
	},
}
```

With `style = "flat"`:

```lua
local GameIds = {
	["player.vips.VIP"] = 67890,
	["shop.items.Coins100"] = 11111,
	["shop.specials.special_offer"] = 12345,
}
```

### Extra entries

Inject pre-existing assets into the generated file without syncing them:

```toml
[codegen.extra]
"passes.legacy_vip" = 1234567
"products.starter_pack" = 9876543
```

These entries are merged alongside synced resources in the output, using the same flat/nested style.

### TypeScript

When `typescript = true`, a `.d.ts` file is generated alongside the Luau module:

```typescript
// This file is auto-generated by rbxsync. Do not edit manually.

declare const GameIds: {
	"badges.Welcome": number
	"passes.VIP": number
	"products.Coins100": number
}

export = GameIds
```

### Key escaping

Resource names that aren't valid Luau identifiers are automatically escaped:

```lua
["my-pass"] = 12345,
```

## Lockfile

rbxsync generates a `rbxsync.lock.toml` that tracks remote state: asset IDs, icon hashes, and metadata. Commit this file to version control.

## Icon Conflict Resolution

When you run `pull` and a remote icon differs from what's in the lockfile:

```
! pass 'VIP': icon differs from remote
  Local:  icons/vip.png (blake3: a1b2c3d4e5f6...)
  Remote: asset 129268487446043
```

Resolve with:
- `--accept-remote` -- Downloads the remote icon to your local path
- `--accept-local` -- Keeps your local icon and re-uploads it on next `sync`

## Attributions

Thank you to [Tarmac](https://github.com/Roblox/tarmac) for the alpha bleeding implementation, which was used in this project.
