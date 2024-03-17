import fs from "fs/promises";
import { join } from "path";

const package_json = JSON.parse(await fs.readFile("package.json", "utf8"));

const cargo_toml = await fs.readFile("../Cargo.toml", "utf8");
const version = cargo_toml
  .split("\n")
  .find((line) => line.includes("version"))
  .split('"')[1];

const platform_packages_dir = "platform_package";

const platform_package_names = await Promise.all(
  (
    await fs.readdir(platform_packages_dir)
  ).map(async (name) => {
    const p = JSON.parse(
      await fs.readFile(
        join(platform_packages_dir, name, "package.json"),
        "utf8"
      )
    );
    if (p.version !== version) {
      console.error(
        name,
        "has a different version than the version of the rpc server.",
        { rpc_server: version, platform_package: p.version }
      );
      throw new Error("version missmatch");
    }
    return { folder_name: name, package_name: p.name };
  })
);

package_json.version = version;
package_json.optionalDependencies = {};
for (const { folder_name, package_name } of platform_package_names) {
  package_json.optionalDependencies[package_name] = version;
}

await fs.writeFile("package.json", JSON.stringify(package_json, null, 4));
