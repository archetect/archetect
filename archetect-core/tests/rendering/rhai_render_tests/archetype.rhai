let context = #{};

context.project_name = prompt("Project Name:");

let dir = Directory("default");
render(dir, context);
