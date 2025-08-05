// Simple demonstration of archive functionality without prompts

let project_name = "demo-project";

// Create a simple file structure
Directory("contents").render(#{
    project_name: project_name
});

// Create archives
display("Creating archives...");

// ZIP archive
zip(project_name, project_name + ".zip");
display("Created " + project_name + ".zip");

// TAR archive
tar(project_name, project_name + ".tar");
display("Created " + project_name + ".tar");

// TAR.GZ archive
tar_gz(project_name, project_name + ".tar.gz");
display("Created " + project_name + ".tar.gz");

display("Archive creation complete!");