extends Label


func _process(delta):
	var dc := get_tree().root.get_render_info(Viewport.RENDER_INFO_DRAW_CALLS_IN_FRAME)
	text = "Draw calls: %d" % dc
