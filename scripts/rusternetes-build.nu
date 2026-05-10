#!/usr/bin/env nu

def main [--force(-f)] {
    let repo_path = if ($env | get -o RUSTERNETES_REPO | is-not-empty) {
        $env.RUSTERNETES_REPO
    } else {
        $"($env.HOME)/git/rusternetes"
    }

    if not ($repo_path | path exists) {
        print $"Error: rusternetes repo not found at ($repo_path)"
        print "Clone it or set RUSTERNETES_REPO to its path."
        exit 1
    }

    print $"Building rusternetes component images from ($repo_path)..."
    print ""

    let components = ["api-server", "scheduler", "controller-manager", "kubelet", "kube-proxy"]

    for component in $components {
        let tag = $"kina-rusternetes-($component)"
        let dockerfile = $"($repo_path)/Dockerfile.($component)"

        if not ($dockerfile | path exists) {
            print $"Error: ($dockerfile) not found"
            exit 1
        }

        if not $force {
            let exists = (do { ^container image inspect $tag } | complete).exit_code == 0
            if $exists {
                print $"  ($tag) already exists, skipping \(pass -f to rebuild\)"
                continue
            }
        }

        print $"Building ($tag)..."
        ^container build --progress plain -c 4 -m 6G -t $tag -f $dockerfile $repo_path
        if $env.LAST_EXIT_CODE != 0 {
            print $"Failed to build ($tag)"
            exit 1
        }
        print $"  ($tag) built successfully"
    }

    print ""
    print "Done. You can now run:"
    print "  mise run kina -- create my-cluster --kubernetes-provider rusternetes --workers 3"
}
