let context = #{};

context.name = prompt("Name:");

let result = render("Hello, {{ name }}!", context);
display(result);
