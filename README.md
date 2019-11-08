# tls_read_hancock_bin

CLI tool for transforming [Hancock terrestrial LiDAR binary polar format](https://bitbucket.org/StevenHancock/libclidar) to 2D image for quick inspection.


## Example

```
$ cargo install tls_3d_to_2d
$ tls_3d_to_2d --help


3d-to-2d 0.1.0

USAGE:
    tls_3d_to_2d.exe [OPTIONS] --output <output> [file]...

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -D, --dist-max <dist-max>    Maximum distance [default: 20.0]
    -d, --dist-min <dist-min>    Minimum distance [default: 0.0]
    -o, --output <output>        Output file name
    -r, --res-az <res-az>        Azimuth pixel resolution [default: 0.2]
    -R, --res-zen <res-zen>      Zenith pixel resolution [default: 0.2]
    -Z, --zen-max <zen-max>      Maximum zenith [default: 120.0]
    -z, --zen-min <zen-min>      Minimum zenith [default: 30.0]

ARGS:
    <file>...    Input file list
```