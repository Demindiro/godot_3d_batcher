extends Label


func _process(delta):
	text = "FPS: %d" % Engine.get_frames_per_second()
