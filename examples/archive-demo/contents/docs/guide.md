# {{ project_name }} Guide

This guide demonstrates the structure of a project that can be archived using Archetect's new archive functions.

## Archive Functions

The following functions are now available in Archetect's Rhai scripting:

### zip(source, destination)
Creates a ZIP archive of the source directory.

```rhai
zip("my-project", "my-project.zip");
```

### tar(source, destination)
Creates a TAR archive of the source directory.

```rhai
tar("my-project", "my-project.tar");
```

### tar_gz(source, destination)
Creates a compressed TAR.GZ archive of the source directory.

```rhai
tar_gz("my-project", "my-project.tar.gz");
```

All functions support both string paths and Path objects as the source parameter.