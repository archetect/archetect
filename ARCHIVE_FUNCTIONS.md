# Archive Functions in Archetect

Archetect now supports creating archives (zip, tar, tar.gz) directly from the Rhai scripting engine. This is useful for packaging generated projects or creating distributable bundles.

## Available Functions

### `zip(source, destination)`

Creates a ZIP archive of a directory.

**Parameters:**
- `source`: String path or Path object - the directory to archive
- `destination`: String path - where to save the zip file

**Example:**
```rhai
// Using string paths
zip("my-project", "my-project.zip");

// Using Path object
let project = Path("my-project");
zip(project, "my-project.zip");
```

### `tar(source, destination)`

Creates a TAR archive of a directory.

**Parameters:**
- `source`: String path or Path object - the directory to archive
- `destination`: String path - where to save the tar file

**Example:**
```rhai
// Using string paths
tar("my-project", "my-project.tar");

// Using Path object
let project = Path("my-project");
tar(project, "my-project.tar");
```

### `tar_gz(source, destination)`

Creates a compressed TAR.GZ archive of a directory.

**Parameters:**
- `source`: String path or Path object - the directory to archive
- `destination`: String path - where to save the tar.gz file

**Example:**
```rhai
// Using string paths
tar_gz("my-project", "my-project.tar.gz");

// Using Path object
let project = Path("my-project");
tar_gz(project, "my-project.tar.gz");
```

## Complete Example

Here's a complete example showing how to generate a project and create archives:

```rhai
// Get project information
let project_name = prompt("text", #{
    message: "What is your project name?",
    defaults_with: "my-project"
});

// Render the project template
Directory("template").render(#{
    name: project_name
});

// Create different archive formats
display("Creating archives...");

// ZIP format
let zip_file = project_name + ".zip";
zip(project_name, zip_file);
display("✓ Created " + zip_file);

// TAR format
let tar_file = project_name + ".tar";
tar(project_name, tar_file);
display("✓ Created " + tar_file);

// Compressed TAR.GZ format
let tar_gz_file = project_name + ".tar.gz";
tar_gz(project_name, tar_gz_file);
display("✓ Created " + tar_gz_file);

display("Project generated and archived successfully!");
```

## Error Handling

The archive functions will fail with appropriate error messages if:
- The source directory doesn't exist
- The source path is not a directory
- There are I/O errors during archive creation
- The destination path contains invalid characters or path manipulation attempts

## Notes

- All paths are relative to the render destination directory
- Parent directories for the destination will be created automatically if they don't exist
- The archive functions preserve the directory structure and file permissions
- ZIP archives use DEFLATE compression
- TAR.GZ archives use gzip compression