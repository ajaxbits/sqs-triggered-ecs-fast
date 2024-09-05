{...}: {
  perSystem = {
    pkgs,
    config,
    ...
  }: let
    crateName = "fastest-ecs-scheduler";
  in {
    nci.projects.${crateName}.path = ./.;
    nci.crates.${crateName} = {};
  };
}
