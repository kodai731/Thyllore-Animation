use std::collections::HashMap;

fn str_ref(s: &ufbx::String) -> &str {
    s.as_ref()
}

#[test]
fn test_compare_fbx_dom() {
    let original_path = "assets/models/stickman/stickman_bin.fbx";
    let exported_path = "assets/exports/Armature-ArmatureAction.fbx";

    let mut orig_opts = ufbx::LoadOpts::default();
    orig_opts.retain_dom = true;
    let mut exp_opts = ufbx::LoadOpts::default();
    exp_opts.retain_dom = true;

    let original = ufbx::load_file(original_path, orig_opts)
        .expect("Failed to load original FBX");
    let exported = ufbx::load_file(exported_path, exp_opts)
        .expect("Failed to load exported FBX");

    let orig_dom = original.dom_root.as_ref().expect("No DOM root in original");
    let exp_dom = exported.dom_root.as_ref().expect("No DOM root in exported");

    println!("\n=== FBX DOM Comparison ===\n");

    println!("--- Original DOM top-level nodes ---");
    for child in orig_dom.children.iter() {
        println!("  {}: {} values, {} children", child.name, child.values.len(), child.children.len());
    }

    println!("\n--- Exported DOM top-level nodes ---");
    for child in exp_dom.children.iter() {
        println!("  {}: {} values, {} children", child.name, child.values.len(), child.children.len());
    }

    let orig_connections = ufbx::dom_find(orig_dom, "Connections");
    let exp_connections = ufbx::dom_find(exp_dom, "Connections");

    if let (Some(orig_conns), Some(exp_conns)) = (orig_connections, exp_connections) {
        println!("\n--- Connection counts ---");
        println!("  Original: {} connections", orig_conns.children.len());
        println!("  Exported: {} connections", exp_conns.children.len());

        println!("\n--- Original OP connections ---");
        for conn in orig_conns.children.iter() {
            if conn.values.len() >= 4 && str_ref(&conn.values[0].value_str) == "OP" {
                let child_uid = conn.values[1].value_int;
                let parent_uid = conn.values[2].value_int;
                let prop = str_ref(&conn.values[3].value_str);
                println!("  C: OP, child={}, parent={}, prop=\"{}\"", child_uid, parent_uid, prop);
            }
        }

        println!("\n--- Exported OP connections ---");
        for conn in exp_conns.children.iter() {
            if conn.values.len() >= 4 && str_ref(&conn.values[0].value_str) == "OP" {
                let child_uid = conn.values[1].value_int;
                let parent_uid = conn.values[2].value_int;
                let prop = str_ref(&conn.values[3].value_str);
                println!("  C: OP, child={}, parent={}, prop=\"{}\"", child_uid, parent_uid, prop);
            }
        }

        println!("\n--- Original OO connections ---");
        for conn in orig_conns.children.iter() {
            if conn.values.len() >= 3 && str_ref(&conn.values[0].value_str) == "OO" {
                let child_uid = conn.values[1].value_int;
                let parent_uid = conn.values[2].value_int;
                println!("  C: OO, child={}, parent={}", child_uid, parent_uid);
            }
        }

        println!("\n--- Exported OO connections ---");
        for conn in exp_conns.children.iter() {
            if conn.values.len() >= 3 && str_ref(&conn.values[0].value_str) == "OO" {
                let child_uid = conn.values[1].value_int;
                let parent_uid = conn.values[2].value_int;
                println!("  C: OO, child={}, parent={}", child_uid, parent_uid);
            }
        }
    }

    let orig_objects = ufbx::dom_find(orig_dom, "Objects");
    let exp_objects = ufbx::dom_find(exp_dom, "Objects");

    if let (Some(orig_objs), Some(exp_objs)) = (orig_objects, exp_objects) {
        println!("\n--- Objects section comparison ---");
        let orig_obj_types = count_object_types(orig_objs);
        let exp_obj_types = count_object_types(exp_objs);

        println!("  Original object types:");
        for (t, c) in &orig_obj_types {
            println!("    {}: {}", t, c);
        }
        println!("  Exported object types:");
        for (t, c) in &exp_obj_types {
            println!("    {}: {}", t, c);
        }

        println!("\n--- Armature Model Properties70 ---");
        println!("  [Original]");
        print_model_properties(orig_objs, "Armature");
        println!("  [Exported]");
        print_model_properties(exp_objs, "Armature");

        println!("\n--- Bone Model Properties70 ---");
        println!("  [Original]");
        print_model_properties(orig_objs, "Bone");
        println!("  [Exported]");
        print_model_properties(exp_objs, "Bone");

        println!("\n--- Mesh Model NurbsPath ---");
        println!("  [Original]");
        print_model_properties(orig_objs, "NurbsPath");
        println!("  [Exported]");
        print_model_properties(exp_objs, "NurbsPath");

        println!("\n--- AnimationCurveNode comparison ---");
        let orig_acns: Vec<_> = orig_objs.children.iter()
            .filter(|c| str_ref(&c.name) == "AnimationCurveNode")
            .collect();
        let exp_acns: Vec<_> = exp_objs.children.iter()
            .filter(|c| str_ref(&c.name) == "AnimationCurveNode")
            .collect();

        println!("  Original AnimationCurveNodes: {}", orig_acns.len());
        println!("  Exported AnimationCurveNodes: {}", exp_acns.len());

        for (i, acn) in orig_acns.iter().enumerate().take(3) {
            println!("\n  Original ACN[{}]:", i);
            print_node_values(acn);
            print_properties70(acn, "    ");
        }

        for (i, acn) in exp_acns.iter().enumerate().take(3) {
            println!("\n  Exported ACN[{}]:", i);
            print_node_values(acn);
            print_properties70(acn, "    ");
        }

        println!("\n--- ACN node names (first 3) ---");
        for (i, acn) in orig_acns.iter().enumerate().take(3) {
            let name_str: &str = &acn.name;
            let display = name_str.replace('\x00', "\\0").replace('\x01', "\\1");
            println!("  Original ACN[{}] node.name = \"{}\"", i, display);
            if acn.values.len() > 1 {
                let val1: &str = &acn.values[1].value_str;
                let display1 = val1.replace('\x00', "\\0").replace('\x01', "\\1");
                println!("  Original ACN[{}] values[1].str = \"{}\"", i, display1);
            }
        }
        for (i, acn) in exp_acns.iter().enumerate().take(3) {
            let name_str: &str = &acn.name;
            let display = name_str.replace('\x00', "\\0").replace('\x01', "\\1");
            println!("  Exported ACN[{}] node.name = \"{}\"", i, display);
            if acn.values.len() > 1 {
                let val1: &str = &acn.values[1].value_str;
                let display1 = val1.replace('\x00', "\\0").replace('\x01', "\\1");
                println!("  Exported ACN[{}] values[1].str = \"{}\"", i, display1);
            }
        }

        println!("\n--- AnimationCurve comparison (first 2) ---");
        let orig_curves: Vec<_> = orig_objs.children.iter()
            .filter(|c| str_ref(&c.name) == "AnimationCurve")
            .collect();
        let exp_curves: Vec<_> = exp_objs.children.iter()
            .filter(|c| str_ref(&c.name) == "AnimationCurve")
            .collect();

        println!("  Original AnimationCurves: {}", orig_curves.len());
        println!("  Exported AnimationCurves: {}", exp_curves.len());

        for (i, curve) in orig_curves.iter().enumerate().take(2) {
            print_anim_curve_summary(curve, &format!("Original[{}]", i));
        }
        for (i, curve) in exp_curves.iter().enumerate().take(2) {
            print_anim_curve_summary(curve, &format!("Exported[{}]", i));
        }

        println!("\n--- AnimationStack ---");
        for stack in orig_objs.children.iter().filter(|c| str_ref(&c.name) == "AnimationStack") {
            println!("  Original AnimStack:");
            print_node_values(stack);
            print_properties70(stack, "    ");
        }
        for stack in exp_objs.children.iter().filter(|c| str_ref(&c.name) == "AnimationStack") {
            println!("  Exported AnimStack:");
            print_node_values(stack);
            print_properties70(stack, "    ");
        }

        println!("\n--- AnimationLayer ---");
        for layer in orig_objs.children.iter().filter(|c| str_ref(&c.name) == "AnimationLayer") {
            println!("  Original AnimLayer:");
            print_node_values(layer);
        }
        for layer in exp_objs.children.iter().filter(|c| str_ref(&c.name) == "AnimationLayer") {
            println!("  Exported AnimLayer:");
            print_node_values(layer);
        }

        println!("\n--- Definitions section ---");
        if let Some(orig_defs) = ufbx::dom_find(orig_dom, "Definitions") {
            println!("  Original Definitions:");
            for child in orig_defs.children.iter() {
                if str_ref(&child.name) == "ObjectType" && !child.values.is_empty() {
                    let mut count = 0i64;
                    for sub in child.children.iter() {
                        if str_ref(&sub.name) == "Count" && !sub.values.is_empty() {
                            count = sub.values[0].value_int;
                        }
                    }
                    println!("    ObjectType: {} (count={})", child.values[0].value_str, count);
                }
            }
        }
        if let Some(exp_defs) = ufbx::dom_find(exp_dom, "Definitions") {
            println!("  Exported Definitions:");
            for child in exp_defs.children.iter() {
                if str_ref(&child.name) == "ObjectType" && !child.values.is_empty() {
                    let mut count = 0i64;
                    for sub in child.children.iter() {
                        if str_ref(&sub.name) == "Count" && !sub.values.is_empty() {
                            count = sub.values[0].value_int;
                        }
                    }
                    println!("    ObjectType: {} (count={})", child.values[0].value_str, count);
                }
            }
        }
    }
}

fn count_object_types(objects: &ufbx::DomNode) -> Vec<(String, usize)> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for child in objects.children.iter() {
        *counts.entry(child.name.to_string()).or_insert(0) += 1;
    }
    let mut sorted: Vec<_> = counts.into_iter().collect();
    sorted.sort_by_key(|(name, _)| name.clone());
    sorted
}

fn print_node_values(node: &ufbx::DomNode) {
    for (i, v) in node.values.iter().enumerate() {
        let s: &str = &v.value_str;
        let str_display = s.replace('\x00', "\\0").replace('\x01', "\\1");
        println!("    values[{}]: type={:?}, str=\"{}\", int={}, float={}", i, v.type_, str_display, v.value_int, v.value_float);
    }
}

fn print_model_properties(objects: &ufbx::DomNode, model_name: &str) {
    for obj in objects.children.iter() {
        if str_ref(&obj.name) != "Model" {
            continue;
        }
        if obj.values.len() >= 2 {
            let name_val: &str = &obj.values[1].value_str;
            let base_name = name_val.split('\x00').next().unwrap_or(name_val);
            if base_name != model_name {
                continue;
            }
        }

        println!("    Model node values:");
        print_node_values(obj);
        print_properties70(obj, "    ");

        for child in obj.children.iter() {
            if str_ref(&child.name) != "Properties70" {
                println!("    Other child: {} ({} values)", child.name, child.values.len());
            }
        }
        return;
    }
    println!("    (not found)");
}

fn print_properties70(node: &ufbx::DomNode, prefix: &str) {
    for child in node.children.iter() {
        if str_ref(&child.name) != "Properties70" {
            continue;
        }
        println!("{}Properties70 ({} properties):", prefix, child.children.len());
        for prop in child.children.iter() {
            let vals: Vec<String> = prop.values.iter().map(|v| {
                match v.type_ {
                    ufbx::DomValueType::String => {
                        let s: &str = &v.value_str;
                        format!("\"{}\"", s)
                    }
                    ufbx::DomValueType::Number => {
                        if v.value_float == (v.value_int as f64) {
                            format!("{}", v.value_int)
                        } else {
                            format!("{:.6}", v.value_float)
                        }
                    }
                    _ => format!("{:?}", v.type_),
                }
            }).collect();
            println!("{}  P: {}", prefix, vals.join(", "));
        }
        return;
    }
    println!("{}(no Properties70)", prefix);
}

fn print_anim_curve_summary(curve: &ufbx::DomNode, label: &str) {
    println!("  {} AnimationCurve:", label);
    print_node_values(curve);
    for child in curve.children.iter() {
        let name: &str = &child.name;
        match name {
            "KeyTime" | "KeyValueFloat" => {
                println!("    {}: {} values", child.name, child.values.len());
                if !child.values.is_empty() {
                    println!("      type={:?}", child.values[0].type_);
                }
            }
            "Default" | "KeyVer" => {
                if !child.values.is_empty() {
                    println!("    {}: {}", child.name, child.values[0].value_float);
                }
            }
            _ => {
                println!("    {}: {} values", child.name, child.values.len());
            }
        }
    }
}
