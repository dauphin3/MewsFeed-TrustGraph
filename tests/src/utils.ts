import { fileURLToPath } from "url";
import path from "path";
import {
  AppBundle,
  AppBundleSource,
  CellProvisioningStrategy,
} from "@holochain/client";
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const createMewsFeedAppBundleSource = (
  properties: any = {}
): AppBundleSource => {
  return {
    bundle: {
      manifest: {
        manifest_version: "1",
        name: "mewsfeed",
        roles: [
          {
            name: "mewsfeed",
            provisioning: {
              strategy: CellProvisioningStrategy.Create,
              deferred: false,
            },
            dna: {
              path: path.join(
                __dirname,
                "../../dnas/mewsfeed/workdir/mewsfeed.dna"
              ),
              modifiers: {
                properties,
              },
            },
          },
        ],
      },
      resources: {},
    },
  };
};

export const mewsfeedAppBundleSource: AppBundleSource =
  createMewsFeedAppBundleSource({
    mew_characters_min: 5,
    mew_characters_max: 200,
    prefix_index_width: 3,
    time_index_chunk_interval_ms: 30000,
  });

export const mewsfeedAppBundleSourceNoLengthLimits: AppBundleSource =
  createMewsFeedAppBundleSource({
    prefix_index_width: 3,
    time_index_chunk_interval_ms: 30000,
  });
