# Packaged Musubi, a real Aizome session and WirePlumber-managed ports.
{ pkgs, sekka, musubiModule, ... }:
pkgs.testers.runNixOSTest {
  name = "musubi-patchbay";
  globalTimeout = 600;
  nodes.machine = { ... }: {
    imports = [ sekka.nixosModules.aizome musubiModule ];
    programs.aizome.enable = true;
    programs.musubi.enable = true;
    users.users.alice = {
      isNormalUser = true;
      uid = 1000;
      password = "test-only";
    };
    services.greetd = {
      enable = true;
      settings.initial_session = {
        user = "alice";
        command = "/run/current-system/sw/bin/aizome-session";
      };
      settings.default_session.command = "${pkgs.greetd}/bin/agreety --cmd /run/current-system/sw/bin/aizome-session";
    };
    services.pipewire = {
      enable = true;
      wireplumber.enable = true;
      alsa.enable = true;
      extraConfig.pipewire."90-musubi-vm-sinks"."context.objects" = map (s: {
        factory = "adapter";
        args = {
          "factory.name" = "support.null-audio-sink";
          "node.name" = s;
          "node.description" = s;
          "media.class" = "Audio/Sink";
          "audio.position" = "FL,FR";
          "monitor.channel-volumes" = true;
        };
      }) [ "musubi-vm-default" "musubi-vm-patch" ];
    };
    boot.kernelModules = [ "snd-seq" "snd-virmidi" "vivid" ];
    environment.systemPackages = [ pkgs.pipewire pkgs.alsa-utils pkgs.jq ];
    # A stream connected to the default sink gets WirePlumber-managed output
    # ports, which can then be patched to the other sink by Musubi.
    systemd.user.services.musubi-vm-player = {
      description = "Endless silent stereo stream for Musubi";
      wantedBy = [ "default.target" ];
      after = [ "pipewire.service" "wireplumber.service" ];
      serviceConfig = {
        ExecStart = "${pkgs.pipewire}/bin/pw-play --raw --format=s16 --rate=48000 --channels=2 --properties={\\\"node.name\\\":\\\"musubi_vm_player\\\",\\\"node.description\\\":\\\"Musubi VM Player\\\"} /dev/zero";
        Restart = "always";
        RestartSec = 2;
      };
    };
    systemd.user.settings.Manager.DefaultEnvironment = "AIZOME_KIT_INSPECT=/run/user/1000/musubi-inspect";
    systemd.user.tmpfiles.rules = [ "d %t/musubi-inspect 0700 - - -" ];
    virtualisation = {
      memorySize = 4096;
      cores = 4;
      resolution = { x = 1280; y = 800; };
      qemu.options = [ "-vga none" "-device virtio-gpu-pci,xres=1280,yres=800" ];
    };
  };
  testScript = builtins.readFile ./vm-test.py;
}
