// Demonstrate the new archive functionality

// Get project name from user
let project_name = prompt("text", #{
    message: "What is your project name?",
    "default": "my-project"
});

// Create a directory structure
Directory("contents").render(#{
    project_name: project_name
});

// Demonstrate different archive formats
display("");
display("Creating archives of the generated project...");

// Create a zip archive
let zip_name = project_name + ".zip";
zip(project_name, zip_name);
display("✓ Created " + zip_name);

// Create a tar archive
let tar_name = project_name + ".tar";
tar(project_name, tar_name);
display("✓ Created " + tar_name);

// Create a compressed tar.gz archive
let tar_gz_name = project_name + ".tar.gz";
tar_gz(project_name, tar_gz_name);
display("✓ Created " + tar_gz_name);

// Demonstrate using Path objects
display("");
display("Using Path objects for archiving...");

let project_path = Path(project_name);
if project_path.exists() {
    zip(project_path, project_name + "-from-path.zip");
    display("✓ Created " + project_name + "-from-path.zip using Path object");
}

display("");
display("Archive generation complete!");
display("The following archives were created:");
display("  - " + zip_name);
display("  - " + tar_name);
display("  - " + tar_gz_name);
display("  - " + project_name + "-from-path.zip");