import pytest


def test_np_zeros_creates_frame():
    """Test that np.zeros with video frame shape returns a Frame."""
    import vidformer.numpy as np
    from vidformer.cv2 import Frame

    # Standard video frame shape (height, width, channels)
    canvas = np.zeros((720, 1280, 3), dtype=np.uint8)
    assert isinstance(canvas, Frame)
    assert canvas.shape == (720, 1280, 3)


def test_np_zeros_grayscale_frame():
    """Test that np.zeros with grayscale shape returns a Frame."""
    import vidformer.numpy as np
    from vidformer.cv2 import Frame

    canvas = np.zeros((480, 640, 1), dtype=np.uint8)
    assert isinstance(canvas, Frame)
    assert canvas.shape == (480, 640, 1)


def test_np_zeros_grayscale_pixel_exact():
    """Test that np.zeros grayscale produces pixel-exact black frames."""
    import vidformer.numpy as vf_np
    import numpy as np

    vf_result = vf_np.zeros((100, 200, 1), dtype=np.uint8).numpy()
    np_result = np.zeros((100, 200, 1), dtype=np.uint8)

    assert vf_result.shape == np_result.shape
    assert np.array_equal(vf_result, np_result)


def test_np_ones_grayscale_frame():
    """Test that np.ones with grayscale shape returns a Frame."""
    import vidformer.numpy as np
    from vidformer.cv2 import Frame

    frame = np.ones((100, 200, 1), dtype=np.uint8)
    assert isinstance(frame, Frame)
    assert frame.shape == (100, 200, 1)


def test_np_ones_grayscale_pixel_exact():
    """Test that np.ones grayscale produces correct pixel values."""
    import vidformer.numpy as vf_np
    import numpy as np

    vf_result = vf_np.ones((100, 200, 1), dtype=np.uint8).numpy()
    np_result = np.ones((100, 200, 1), dtype=np.uint8)

    assert vf_result.shape == np_result.shape
    assert np.array_equal(vf_result, np_result)


def test_np_full_grayscale_frame():
    """Test that np.full with grayscale shape returns a Frame."""
    import vidformer.numpy as np
    from vidformer.cv2 import Frame

    frame = np.full((100, 200, 1), 128, dtype=np.uint8)
    assert isinstance(frame, Frame)
    assert frame.shape == (100, 200, 1)


def test_np_full_grayscale_pixel_exact():
    """Test that np.full grayscale produces correct pixel values."""
    import vidformer.numpy as vf_np
    import numpy as np

    vf_result = vf_np.full((100, 200, 1), 128, dtype=np.uint8).numpy()
    np_result = np.full((100, 200, 1), 128, dtype=np.uint8)

    assert vf_result.shape == np_result.shape
    assert np.array_equal(vf_result, np_result)


def test_grayscale_zeros_like():
    """Test zeros_like with a grayscale Frame input."""
    import vidformer.numpy as np
    from vidformer.cv2 import Frame

    original = np.full((100, 200, 1), 64, dtype=np.uint8)
    result = np.zeros_like(original)

    assert isinstance(result, Frame)
    assert result.shape == (100, 200, 1)
    assert result.numpy().sum() == 0


def test_grayscale_ones_like():
    """Test ones_like with a grayscale Frame input."""
    import vidformer.numpy as np
    from vidformer.cv2 import Frame

    original = np.zeros((100, 200, 1), dtype=np.uint8)
    result = np.ones_like(original)

    assert isinstance(result, Frame)
    assert result.shape == (100, 200, 1)
    assert (result.numpy() == 1).all()


def test_grayscale_full_like():
    """Test full_like with a grayscale Frame input."""
    import vidformer.numpy as np
    from vidformer.cv2 import Frame

    original = np.zeros((100, 200, 1), dtype=np.uint8)
    result = np.full_like(original, 200)

    assert isinstance(result, Frame)
    assert result.shape == (100, 200, 1)
    assert (result.numpy() == 200).all()


def test_grayscale_flip():
    """Test flip with grayscale frames returns correct shape."""
    import vidformer.numpy as np
    from vidformer.cv2 import Frame

    frame = np.full((100, 200, 1), 128, dtype=np.uint8)
    result = np.flip(frame, axis=1)

    assert isinstance(result, Frame)
    assert result.shape == (100, 200, 1)


def test_grayscale_rot90():
    """Test rot90 with grayscale frames returns correct shape."""
    import vidformer.numpy as np
    from vidformer.cv2 import Frame

    frame = np.full((100, 200, 1), 128, dtype=np.uint8)
    result = np.rot90(frame, k=1)

    assert isinstance(result, Frame)
    # 90° rotation swaps dimensions
    assert result.shape == (200, 100, 1)


def test_np_zeros_default_dtype():
    """Test that np.zeros with no dtype defaults to uint8 for frame shapes."""
    import vidformer.numpy as np
    from vidformer.cv2 import Frame

    canvas = np.zeros((100, 200, 3))
    assert isinstance(canvas, Frame)


def test_np_zeros_non_frame_shape():
    """Test that np.zeros with non-frame shape returns numpy array."""
    import vidformer.numpy as np
    import numpy as real_np
    from vidformer.cv2 import Frame

    # 2D array - not a frame
    arr = np.zeros((100, 200))
    assert not isinstance(arr, Frame)
    assert isinstance(arr, real_np.ndarray)

    # 1D array - not a frame
    arr = np.zeros((100,))
    assert not isinstance(arr, Frame)


def test_np_zeros_non_uint8_dtype():
    """Test that np.zeros with non-uint8 dtype returns numpy array."""
    import vidformer.numpy as np
    import numpy as real_np
    from vidformer.cv2 import Frame

    arr = np.zeros((100, 200, 3), dtype=np.float32)
    assert not isinstance(arr, Frame)
    assert isinstance(arr, real_np.ndarray)


def test_np_hstack_with_frames():
    """Test that np.hstack with Frames uses cv2.hconcat."""
    import vidformer.numpy as np
    from vidformer.cv2 import Frame

    frame1 = np.zeros((100, 200, 3), dtype=np.uint8)
    frame2 = np.zeros((100, 300, 3), dtype=np.uint8)

    result = np.hstack([frame1, frame2])
    assert isinstance(result, Frame)
    assert result.shape == (100, 500, 3)


def test_np_hstack_with_numpy_arrays():
    """Test that np.hstack with numpy arrays falls back to numpy."""
    import vidformer.numpy as np
    import numpy as real_np
    from vidformer.cv2 import Frame

    arr1 = real_np.array([[1, 2], [3, 4]])
    arr2 = real_np.array([[5, 6], [7, 8]])

    result = np.hstack([arr1, arr2])
    assert not isinstance(result, Frame)
    assert isinstance(result, real_np.ndarray)
    assert result.shape == (2, 4)


def test_np_vstack_with_frames():
    """Test that np.vstack with Frames uses cv2.vconcat."""
    import vidformer.numpy as np
    from vidformer.cv2 import Frame

    frame1 = np.zeros((100, 200, 3), dtype=np.uint8)
    frame2 = np.zeros((150, 200, 3), dtype=np.uint8)

    result = np.vstack([frame1, frame2])
    assert isinstance(result, Frame)
    assert result.shape == (250, 200, 3)


def test_np_vstack_with_numpy_arrays():
    """Test that np.vstack with numpy arrays falls back to numpy."""
    import vidformer.numpy as np
    import numpy as real_np
    from vidformer.cv2 import Frame

    arr1 = real_np.array([[1, 2, 3]])
    arr2 = real_np.array([[4, 5, 6]])

    result = np.vstack([arr1, arr2])
    assert not isinstance(result, Frame)
    assert isinstance(result, real_np.ndarray)
    assert result.shape == (2, 3)


def test_np_passthrough_other_functions():
    """Test that other numpy functions are passed through."""
    import vidformer.numpy as np
    import numpy as real_np

    # Test array creation
    arr = np.array([1, 2, 3])
    assert isinstance(arr, real_np.ndarray)

    # Test arange
    arr = np.arange(10)
    assert isinstance(arr, real_np.ndarray)
    assert len(arr) == 10

    # Test linspace
    arr = np.linspace(0, 1, 5)
    assert isinstance(arr, real_np.ndarray)
    assert len(arr) == 5


def test_np_dtype_passthrough():
    """Test that numpy dtypes are accessible."""
    import vidformer.numpy as np
    import numpy as real_np

    assert np.uint8 == real_np.uint8
    assert np.float32 == real_np.float32
    assert np.int64 == real_np.int64


def test_grid_layout_pattern():
    """Test the common 2x2 grid layout pattern that LLMs often generate."""
    import vidformer.numpy as np
    from vidformer.cv2 import Frame, resize

    # Create 4 frames of different sizes
    frame1 = np.zeros((100, 100, 3), dtype=np.uint8)
    frame2 = np.zeros((100, 100, 3), dtype=np.uint8)
    frame3 = np.zeros((100, 100, 3), dtype=np.uint8)
    frame4 = np.zeros((100, 100, 3), dtype=np.uint8)

    # Create 2x2 grid using hstack and vstack
    top_row = np.hstack([frame1, frame2])
    bottom_row = np.hstack([frame3, frame4])
    grid = np.vstack([top_row, bottom_row])

    assert isinstance(grid, Frame)
    assert grid.shape == (200, 200, 3)


def test_canvas_with_slice_assignment():
    """Test creating a canvas and assigning frames to regions."""
    import vidformer.numpy as np
    from vidformer.cv2 import Frame

    # Create a canvas
    canvas = np.zeros((720, 1280, 3), dtype=np.uint8)
    assert isinstance(canvas, Frame)

    # Create a smaller frame
    small_frame = np.zeros((360, 640, 3), dtype=np.uint8)

    # Assign to a region (top-left quadrant)
    canvas[0:360, 0:640] = small_frame

    # Canvas should still be a Frame
    assert isinstance(canvas, Frame)
    assert canvas.shape == (720, 1280, 3)


def test_mixed_frame_and_numpy_hstack():
    """Test hstack with a mix of Frames - all must be Frames for Frame output."""
    import vidformer.numpy as np
    from vidformer.cv2 import Frame

    frame = np.zeros((100, 100, 3), dtype=np.uint8)
    # Both are frames (np.zeros returns Frame for this shape)
    frame2 = np.zeros((100, 100, 3), dtype=np.uint8)

    result = np.hstack([frame, frame2])
    assert isinstance(result, Frame)
    assert result.shape == (100, 200, 3)


def test_video_side_by_side_pattern():
    """Test the common side-by-side video comparison pattern."""
    import vidformer.numpy as np
    from vidformer.cv2 import Frame, resize

    # Simulate two video frames
    video1_frame = np.zeros((480, 640, 3), dtype=np.uint8)
    video2_frame = np.zeros((480, 640, 3), dtype=np.uint8)

    # Scale both to half width
    scaled1 = resize(video1_frame, (320, 480))
    scaled2 = resize(video2_frame, (320, 480))

    # Combine side by side
    combined = np.hstack([scaled1, scaled2])

    assert isinstance(combined, Frame)
    assert combined.shape == (480, 640, 3)


def test_video_stack_vertical_pattern():
    """Test stacking videos vertically (e.g., before/after comparison)."""
    import vidformer.numpy as np
    from vidformer.cv2 import Frame, resize

    # Simulate before/after frames
    before_frame = np.zeros((360, 640, 3), dtype=np.uint8)
    after_frame = np.zeros((360, 640, 3), dtype=np.uint8)

    # Stack vertically
    stacked = np.vstack([before_frame, after_frame])

    assert isinstance(stacked, Frame)
    assert stacked.shape == (720, 640, 3)


# =============================================================================
# Tests for concatenate
# =============================================================================


def test_np_concatenate_axis0_with_frames():
    """Test that np.concatenate with axis=0 uses cv2.vconcat for Frames."""
    import vidformer.numpy as np
    from vidformer.cv2 import Frame

    frame1 = np.zeros((100, 200, 3), dtype=np.uint8)
    frame2 = np.zeros((150, 200, 3), dtype=np.uint8)

    result = np.concatenate([frame1, frame2], axis=0)
    assert isinstance(result, Frame)
    assert result.shape == (250, 200, 3)


def test_np_concatenate_axis1_with_frames():
    """Test that np.concatenate with axis=1 uses cv2.hconcat for Frames."""
    import vidformer.numpy as np
    from vidformer.cv2 import Frame

    frame1 = np.zeros((100, 200, 3), dtype=np.uint8)
    frame2 = np.zeros((100, 300, 3), dtype=np.uint8)

    result = np.concatenate([frame1, frame2], axis=1)
    assert isinstance(result, Frame)
    assert result.shape == (100, 500, 3)


def test_np_concatenate_with_numpy_arrays():
    """Test that np.concatenate with numpy arrays falls back to numpy."""
    import vidformer.numpy as np
    import numpy as real_np
    from vidformer.cv2 import Frame

    arr1 = real_np.array([[1, 2], [3, 4]])
    arr2 = real_np.array([[5, 6], [7, 8]])

    result = np.concatenate([arr1, arr2], axis=0)
    assert not isinstance(result, Frame)
    assert isinstance(result, real_np.ndarray)
    assert result.shape == (4, 2)


# =============================================================================
# Tests for flip
# =============================================================================


def test_np_flip_horizontal_with_frame():
    """Test that np.flip with axis=1 uses cv2.flip for horizontal flip."""
    import vidformer.numpy as np
    from vidformer.cv2 import Frame

    frame = np.zeros((100, 200, 3), dtype=np.uint8)
    result = np.flip(frame, axis=1)

    assert isinstance(result, Frame)
    assert result.shape == (100, 200, 3)


def test_np_flip_vertical_with_frame():
    """Test that np.flip with axis=0 uses cv2.flip for vertical flip."""
    import vidformer.numpy as np
    from vidformer.cv2 import Frame

    frame = np.zeros((100, 200, 3), dtype=np.uint8)
    result = np.flip(frame, axis=0)

    assert isinstance(result, Frame)
    assert result.shape == (100, 200, 3)


def test_np_flip_both_axes_with_frame():
    """Test that np.flip with axis=None flips both axes."""
    import vidformer.numpy as np
    from vidformer.cv2 import Frame

    frame = np.zeros((100, 200, 3), dtype=np.uint8)
    result = np.flip(frame)

    assert isinstance(result, Frame)
    assert result.shape == (100, 200, 3)


def test_np_flip_with_numpy_array():
    """Test that np.flip with numpy arrays falls back to numpy."""
    import vidformer.numpy as np
    import numpy as real_np
    from vidformer.cv2 import Frame

    arr = real_np.array([[1, 2], [3, 4]])
    result = np.flip(arr, axis=1)

    assert not isinstance(result, Frame)
    assert isinstance(result, real_np.ndarray)
    assert real_np.array_equal(result, real_np.array([[2, 1], [4, 3]]))


# =============================================================================
# Tests for rot90
# =============================================================================


def test_np_rot90_once_with_frame():
    """Test that np.rot90 with k=1 rotates 90° counter-clockwise."""
    import vidformer.numpy as np
    from vidformer.cv2 import Frame

    frame = np.zeros((100, 200, 3), dtype=np.uint8)
    result = np.rot90(frame)

    assert isinstance(result, Frame)
    # 90° rotation swaps dimensions
    assert result.shape == (200, 100, 3)


def test_np_rot90_twice_with_frame():
    """Test that np.rot90 with k=2 rotates 180°."""
    import vidformer.numpy as np
    from vidformer.cv2 import Frame

    frame = np.zeros((100, 200, 3), dtype=np.uint8)
    result = np.rot90(frame, k=2)

    assert isinstance(result, Frame)
    # 180° rotation preserves dimensions
    assert result.shape == (100, 200, 3)


def test_np_rot90_thrice_with_frame():
    """Test that np.rot90 with k=3 rotates 270° counter-clockwise (= 90° clockwise)."""
    import vidformer.numpy as np
    from vidformer.cv2 import Frame

    frame = np.zeros((100, 200, 3), dtype=np.uint8)
    result = np.rot90(frame, k=3)

    assert isinstance(result, Frame)
    # 270° rotation swaps dimensions
    assert result.shape == (200, 100, 3)


def test_np_rot90_zero_with_frame():
    """Test that np.rot90 with k=0 returns the same frame."""
    import vidformer.numpy as np
    from vidformer.cv2 import Frame

    frame = np.zeros((100, 200, 3), dtype=np.uint8)
    result = np.rot90(frame, k=0)

    assert isinstance(result, Frame)
    assert result.shape == (100, 200, 3)


def test_np_rot90_with_numpy_array():
    """Test that np.rot90 with numpy arrays falls back to numpy."""
    import vidformer.numpy as np
    import numpy as real_np
    from vidformer.cv2 import Frame

    arr = real_np.array([[1, 2], [3, 4]])
    result = np.rot90(arr)

    assert not isinstance(result, Frame)
    assert isinstance(result, real_np.ndarray)
    assert result.shape == (2, 2)


# =============================================================================
# Pixel-exact compatibility tests
# =============================================================================


def test_zeros_pixel_exact():
    """Test that zeros produces pixel-exact black frames."""
    import vidformer.numpy as vf_np
    import numpy as np

    vf_result = vf_np.zeros((100, 200, 3), dtype=np.uint8).numpy()
    np_result = np.zeros((100, 200, 3), dtype=np.uint8)

    assert np.array_equal(vf_result, np_result)


def test_hstack_pixel_exact():
    """Test that hstack produces pixel-exact results matching numpy."""
    import vidformer.numpy as vf_np
    import vidformer.cv2 as vf_cv2
    import numpy as np

    # Create test images with different colors
    img1 = np.full((100, 50, 3), (255, 0, 0), dtype=np.uint8)
    img2 = np.full((100, 75, 3), (0, 255, 0), dtype=np.uint8)

    np_result = np.hstack([img1, img2])
    vf_result = vf_np.hstack([vf_cv2.frameify(img1), vf_cv2.frameify(img2)]).numpy()

    assert vf_result.shape == np_result.shape
    assert np.allclose(vf_result, np_result, atol=1)


def test_vstack_pixel_exact():
    """Test that vstack produces pixel-exact results matching numpy."""
    import vidformer.numpy as vf_np
    import vidformer.cv2 as vf_cv2
    import numpy as np

    # Create test images with different colors
    img1 = np.full((50, 100, 3), (255, 0, 0), dtype=np.uint8)
    img2 = np.full((75, 100, 3), (0, 255, 0), dtype=np.uint8)

    np_result = np.vstack([img1, img2])
    vf_result = vf_np.vstack([vf_cv2.frameify(img1), vf_cv2.frameify(img2)]).numpy()

    assert vf_result.shape == np_result.shape
    assert np.allclose(vf_result, np_result, atol=1)


def test_concatenate_axis0_pixel_exact():
    """Test that concatenate axis=0 produces pixel-exact results."""
    import vidformer.numpy as vf_np
    import vidformer.cv2 as vf_cv2
    import numpy as np

    img1 = np.full((50, 100, 3), (100, 150, 200), dtype=np.uint8)
    img2 = np.full((60, 100, 3), (200, 100, 50), dtype=np.uint8)

    np_result = np.concatenate([img1, img2], axis=0)
    vf_result = vf_np.concatenate(
        [vf_cv2.frameify(img1), vf_cv2.frameify(img2)], axis=0
    ).numpy()

    assert vf_result.shape == np_result.shape
    assert np.allclose(vf_result, np_result, atol=1)


def test_concatenate_axis1_pixel_exact():
    """Test that concatenate axis=1 produces pixel-exact results."""
    import vidformer.numpy as vf_np
    import vidformer.cv2 as vf_cv2
    import numpy as np

    img1 = np.full((100, 50, 3), (100, 150, 200), dtype=np.uint8)
    img2 = np.full((100, 60, 3), (200, 100, 50), dtype=np.uint8)

    np_result = np.concatenate([img1, img2], axis=1)
    vf_result = vf_np.concatenate(
        [vf_cv2.frameify(img1), vf_cv2.frameify(img2)], axis=1
    ).numpy()

    assert vf_result.shape == np_result.shape
    assert np.allclose(vf_result, np_result, atol=1)


def test_flip_horizontal_pixel_exact():
    """Test that horizontal flip produces pixel-exact results."""
    import vidformer.numpy as vf_np
    import vidformer.cv2 as vf_cv2
    import numpy as np

    # Create gradient image for flip verification
    img = np.zeros((100, 200, 3), dtype=np.uint8)
    for i in range(200):
        img[:, i, :] = i  # Horizontal gradient

    np_result = np.flip(img, axis=1)
    vf_result = vf_np.flip(vf_cv2.frameify(img), axis=1).numpy()

    assert vf_result.shape == np_result.shape
    assert np.allclose(vf_result, np_result, atol=1)


def test_flip_vertical_pixel_exact():
    """Test that vertical flip produces pixel-exact results."""
    import vidformer.numpy as vf_np
    import vidformer.cv2 as vf_cv2
    import numpy as np

    # Create gradient image for flip verification
    img = np.zeros((100, 200, 3), dtype=np.uint8)
    for i in range(100):
        img[i, :, :] = i  # Vertical gradient

    np_result = np.flip(img, axis=0)
    vf_result = vf_np.flip(vf_cv2.frameify(img), axis=0).numpy()

    assert vf_result.shape == np_result.shape
    assert np.allclose(vf_result, np_result, atol=1)


def test_flip_both_pixel_exact():
    """Test that flip both spatial axes produces correct results.

    Note: np.flip() with axis=None flips ALL axes including channels,
    but for video frames, axis=None means both spatial axes only.
    We compare against flipping axis 0 then axis 1.
    """
    import vidformer.numpy as vf_np
    import vidformer.cv2 as vf_cv2
    import numpy as np

    # Create image with unique pixel at corner
    img = np.zeros((100, 200, 3), dtype=np.uint8)
    img[0, 0] = (255, 0, 0)
    img[99, 199] = (0, 255, 0)

    # For frames, axis=None means flip both spatial axes (not channels)
    np_result = np.flip(np.flip(img, axis=0), axis=1)
    vf_result = vf_np.flip(vf_cv2.frameify(img)).numpy()

    assert vf_result.shape == np_result.shape
    assert np.allclose(vf_result, np_result, atol=1)


def test_rot90_k1_pixel_exact():
    """Test that rot90 k=1 produces pixel-exact results."""
    import vidformer.numpy as vf_np
    import vidformer.cv2 as vf_cv2
    import numpy as np

    # Create image with corner markers
    img = np.zeros((100, 200, 3), dtype=np.uint8)
    img[0, 0] = (255, 0, 0)  # Top-left: red
    img[0, 199] = (0, 255, 0)  # Top-right: green
    img[99, 0] = (0, 0, 255)  # Bottom-left: blue
    img[99, 199] = (255, 255, 0)  # Bottom-right: yellow

    np_result = np.rot90(img, k=1)
    vf_result = vf_np.rot90(vf_cv2.frameify(img), k=1).numpy()

    assert vf_result.shape == np_result.shape
    assert np.allclose(vf_result, np_result, atol=1)


def test_rot90_k2_pixel_exact():
    """Test that rot90 k=2 produces pixel-exact results."""
    import vidformer.numpy as vf_np
    import vidformer.cv2 as vf_cv2
    import numpy as np

    img = np.zeros((100, 200, 3), dtype=np.uint8)
    img[0, 0] = (255, 0, 0)
    img[99, 199] = (0, 255, 0)

    np_result = np.rot90(img, k=2)
    vf_result = vf_np.rot90(vf_cv2.frameify(img), k=2).numpy()

    assert vf_result.shape == np_result.shape
    assert np.allclose(vf_result, np_result, atol=1)


def test_rot90_k3_pixel_exact():
    """Test that rot90 k=3 produces pixel-exact results."""
    import vidformer.numpy as vf_np
    import vidformer.cv2 as vf_cv2
    import numpy as np

    img = np.zeros((100, 200, 3), dtype=np.uint8)
    img[0, 0] = (255, 0, 0)
    img[0, 199] = (0, 255, 0)

    np_result = np.rot90(img, k=3)
    vf_result = vf_np.rot90(vf_cv2.frameify(img), k=3).numpy()

    assert vf_result.shape == np_result.shape
    assert np.allclose(vf_result, np_result, atol=1)


def test_grid_2x2_pixel_exact():
    """Test that 2x2 grid layout produces pixel-exact results."""
    import vidformer.numpy as vf_np
    import vidformer.cv2 as vf_cv2
    import numpy as np

    # Create 4 different colored squares
    red = np.full((50, 50, 3), (255, 0, 0), dtype=np.uint8)
    green = np.full((50, 50, 3), (0, 255, 0), dtype=np.uint8)
    blue = np.full((50, 50, 3), (0, 0, 255), dtype=np.uint8)
    yellow = np.full((50, 50, 3), (255, 255, 0), dtype=np.uint8)

    # Numpy version
    np_top = np.hstack([red, green])
    np_bottom = np.hstack([blue, yellow])
    np_result = np.vstack([np_top, np_bottom])

    # Vidformer version
    vf_top = vf_np.hstack([vf_cv2.frameify(red), vf_cv2.frameify(green)])
    vf_bottom = vf_np.hstack([vf_cv2.frameify(blue), vf_cv2.frameify(yellow)])
    vf_result = vf_np.vstack([vf_top, vf_bottom]).numpy()

    assert vf_result.shape == np_result.shape
    assert np.allclose(vf_result, np_result, atol=1)


# =============================================================================
# Tests for ones and full
# =============================================================================


def test_np_ones_creates_frame():
    """Test that np.ones with video frame shape returns a Frame."""
    import vidformer.numpy as np
    from vidformer.cv2 import Frame

    frame = np.ones((100, 200, 3), dtype=np.uint8)
    assert isinstance(frame, Frame)
    assert frame.shape == (100, 200, 3)


def test_np_ones_pixel_exact():
    """Test that np.ones produces correct pixel values."""
    import vidformer.numpy as vf_np
    import numpy as np

    vf_result = vf_np.ones((100, 200, 3), dtype=np.uint8).numpy()
    np_result = np.ones((100, 200, 3), dtype=np.uint8)

    assert vf_result.shape == np_result.shape
    assert np.array_equal(vf_result, np_result)


def test_np_full_scalar_creates_frame():
    """Test that np.full with scalar fill creates a Frame."""
    import vidformer.numpy as np
    from vidformer.cv2 import Frame

    frame = np.full((100, 200, 3), 128, dtype=np.uint8)
    assert isinstance(frame, Frame)
    assert frame.shape == (100, 200, 3)


def test_np_full_scalar_pixel_exact():
    """Test that np.full with scalar produces correct pixel values."""
    import vidformer.numpy as vf_np
    import numpy as np

    vf_result = vf_np.full((100, 200, 3), 128, dtype=np.uint8).numpy()
    np_result = np.full((100, 200, 3), 128, dtype=np.uint8)

    assert vf_result.shape == np_result.shape
    assert np.array_equal(vf_result, np_result)


def test_np_full_color_creates_frame():
    """Test that np.full with color tuple creates a Frame."""
    import vidformer.numpy as np
    from vidformer.cv2 import Frame

    frame = np.full((100, 200, 3), (255, 128, 64), dtype=np.uint8)
    assert isinstance(frame, Frame)
    assert frame.shape == (100, 200, 3)


def test_np_full_color_pixel_exact():
    """Test that np.full with color tuple produces correct pixel values."""
    import vidformer.numpy as vf_np
    import numpy as np

    # BGR color
    color = (255, 128, 64)
    vf_result = vf_np.full((100, 200, 3), color, dtype=np.uint8).numpy()
    np_result = np.full((100, 200, 3), color, dtype=np.uint8)

    assert vf_result.shape == np_result.shape
    assert np.allclose(vf_result, np_result, atol=1)


def test_np_full_white_frame():
    """Test creating a white frame with np.full."""
    import vidformer.numpy as vf_np
    import numpy as np

    vf_result = vf_np.full((100, 200, 3), 255, dtype=np.uint8).numpy()
    np_result = np.full((100, 200, 3), 255, dtype=np.uint8)

    assert np.array_equal(vf_result, np_result)


def test_np_full_fallback_to_numpy():
    """Test that np.full with non-frame shape falls back to numpy."""
    import vidformer.numpy as np
    import numpy as real_np
    from vidformer.cv2 import Frame

    # 2D array - should fall back to numpy
    arr = np.full((100, 200), 42)
    assert not isinstance(arr, Frame)
    assert isinstance(arr, real_np.ndarray)


# =============================================================================
# Tests for slice assignment with color
# =============================================================================


def test_slice_assign_color():
    """Test assigning a color to a slice of a frame."""
    import vidformer.numpy as np
    from vidformer.cv2 import Frame

    canvas = np.zeros((100, 200, 3), dtype=np.uint8)
    canvas[10:50, 20:80] = (255, 0, 0)  # Red rectangle

    assert isinstance(canvas, Frame)
    result = canvas.numpy()

    # Check that the slice region is red
    assert result[30, 50, 0] == 255  # B
    assert result[30, 50, 1] == 0  # G
    assert result[30, 50, 2] == 0  # R (BGR format from numpy())

    # Check that outside the slice is still black
    assert result[5, 5, 0] == 0
    assert result[5, 5, 1] == 0
    assert result[5, 5, 2] == 0


def test_slice_assign_color_pixel_exact():
    """Test that slice assignment with color matches numpy behavior."""
    import vidformer.numpy as vf_np
    import vidformer.cv2 as vf_cv2
    import numpy as np

    # Numpy version
    np_canvas = np.zeros((100, 200, 3), dtype=np.uint8)
    np_canvas[10:50, 20:80] = (100, 150, 200)

    # Vidformer version
    vf_canvas = vf_np.zeros((100, 200, 3), dtype=np.uint8)
    vf_canvas[10:50, 20:80] = (100, 150, 200)
    vf_result = vf_canvas.numpy()

    assert vf_result.shape == np_canvas.shape
    assert np.allclose(vf_result, np_canvas, atol=1)


def test_grid_with_full():
    """Test creating a 2x2 grid using np.full (symbolic, no frameify)."""
    import vidformer.numpy as np
    from vidformer.cv2 import Frame

    # Create 4 colored squares using np.full (symbolic)
    red = np.full((50, 50, 3), (0, 0, 255), dtype=np.uint8)
    green = np.full((50, 50, 3), (0, 255, 0), dtype=np.uint8)
    blue = np.full((50, 50, 3), (255, 0, 0), dtype=np.uint8)
    yellow = np.full((50, 50, 3), (0, 255, 255), dtype=np.uint8)

    # All should be Frames
    assert isinstance(red, Frame)
    assert isinstance(green, Frame)
    assert isinstance(blue, Frame)
    assert isinstance(yellow, Frame)

    # Create grid
    top_row = np.hstack([red, green])
    bottom_row = np.hstack([blue, yellow])
    grid = np.vstack([top_row, bottom_row])

    assert isinstance(grid, Frame)
    assert grid.shape == (100, 100, 3)


# =============================================================================
# Tests for *_like functions
# =============================================================================


def test_zeros_like_frame():
    """Test zeros_like with a Frame input."""
    import vidformer.numpy as np
    from vidformer.cv2 import Frame

    original = np.full((100, 200, 3), (128, 64, 32), dtype=np.uint8)
    result = np.zeros_like(original)

    assert isinstance(result, Frame)
    assert result.shape == original.shape

    # Verify it's all zeros
    arr = result.numpy()
    assert arr.sum() == 0


def test_zeros_like_numpy():
    """Test zeros_like with a numpy array input."""
    import vidformer.numpy as np
    import numpy as real_np
    from vidformer.cv2 import Frame

    original = real_np.array([[1, 2], [3, 4]])
    result = np.zeros_like(original)

    assert not isinstance(result, Frame)
    assert isinstance(result, real_np.ndarray)
    assert result.shape == original.shape
    assert result.sum() == 0


def test_ones_like_frame():
    """Test ones_like with a Frame input."""
    import vidformer.numpy as np
    from vidformer.cv2 import Frame

    original = np.zeros((100, 200, 3), dtype=np.uint8)
    result = np.ones_like(original)

    assert isinstance(result, Frame)
    assert result.shape == original.shape

    # Verify it's all ones
    arr = result.numpy()
    assert (arr == 1).all()


def test_ones_like_numpy():
    """Test ones_like with a numpy array input."""
    import vidformer.numpy as np
    import numpy as real_np
    from vidformer.cv2 import Frame

    original = real_np.array([[1, 2], [3, 4]])
    result = np.ones_like(original)

    assert not isinstance(result, Frame)
    assert isinstance(result, real_np.ndarray)
    assert (result == 1).all()


def test_full_like_frame():
    """Test full_like with a Frame input."""
    import vidformer.numpy as np
    from vidformer.cv2 import Frame

    original = np.zeros((100, 200, 3), dtype=np.uint8)
    result = np.full_like(original, 128)

    assert isinstance(result, Frame)
    assert result.shape == original.shape

    # Verify it's all 128
    arr = result.numpy()
    assert (arr == 128).all()


def test_full_like_frame_color():
    """Test full_like with a Frame input and color tuple."""
    import vidformer.numpy as np
    from vidformer.cv2 import Frame

    original = np.zeros((100, 200, 3), dtype=np.uint8)
    result = np.full_like(original, (100, 150, 200))

    assert isinstance(result, Frame)
    assert result.shape == original.shape


def test_full_like_numpy():
    """Test full_like with a numpy array input."""
    import vidformer.numpy as np
    import numpy as real_np
    from vidformer.cv2 import Frame

    original = real_np.array([[1, 2], [3, 4]])
    result = np.full_like(original, 42)

    assert not isinstance(result, Frame)
    assert isinstance(result, real_np.ndarray)
    assert (result == 42).all()


def test_zeros_like_pixel_exact():
    """Test zeros_like produces pixel-exact results."""
    import vidformer.numpy as vf_np
    import vidformer.cv2 as vf_cv2
    import numpy as np

    # Create a frame and get zeros_like
    original = np.full((100, 200, 3), (50, 100, 150), dtype=np.uint8)
    vf_original = vf_cv2.frameify(original)

    vf_result = vf_np.zeros_like(vf_original).numpy()
    np_result = np.zeros_like(original)

    assert np.array_equal(vf_result, np_result)


def test_ones_like_pixel_exact():
    """Test ones_like produces pixel-exact results."""
    import vidformer.numpy as vf_np
    import vidformer.cv2 as vf_cv2
    import numpy as np

    original = np.full((100, 200, 3), (50, 100, 150), dtype=np.uint8)
    vf_original = vf_cv2.frameify(original)

    vf_result = vf_np.ones_like(vf_original).numpy()
    np_result = np.ones_like(original)

    assert np.array_equal(vf_result, np_result)


def test_full_like_pixel_exact():
    """Test full_like produces pixel-exact results."""
    import vidformer.numpy as vf_np
    import vidformer.cv2 as vf_cv2
    import numpy as np

    original = np.full((100, 200, 3), (50, 100, 150), dtype=np.uint8)
    vf_original = vf_cv2.frameify(original)

    vf_result = vf_np.full_like(vf_original, 200).numpy()
    np_result = np.full_like(original, 200)

    assert np.array_equal(vf_result, np_result)


# =============================================================================
# Tests for numpy passthrough (non-Frame operations)
# =============================================================================


def test_numpy_array_creation():
    """Test that standard numpy array creation works."""
    import vidformer.numpy as np
    import numpy as real_np

    # array from list
    arr = np.array([1, 2, 3, 4, 5])
    assert isinstance(arr, real_np.ndarray)
    assert real_np.array_equal(arr, real_np.array([1, 2, 3, 4, 5]))

    # arange
    arr = np.arange(0, 10, 2)
    assert real_np.array_equal(arr, real_np.array([0, 2, 4, 6, 8]))

    # linspace
    arr = np.linspace(0, 1, 5)
    assert real_np.allclose(arr, real_np.array([0, 0.25, 0.5, 0.75, 1.0]))

    # eye (identity matrix)
    arr = np.eye(3)
    assert arr.shape == (3, 3)
    assert arr[0, 0] == 1.0
    assert arr[0, 1] == 0.0


def test_numpy_math_operations():
    """Test that numpy math operations work."""
    import vidformer.numpy as np
    import numpy as real_np

    a = np.array([1, 2, 3, 4])
    b = np.array([5, 6, 7, 8])

    # Basic arithmetic
    assert real_np.array_equal(a + b, real_np.array([6, 8, 10, 12]))
    assert real_np.array_equal(a * b, real_np.array([5, 12, 21, 32]))
    assert real_np.array_equal(a - b, real_np.array([-4, -4, -4, -4]))

    # Aggregations
    assert np.sum(a) == 10
    assert np.mean(a) == 2.5
    assert np.max(a) == 4
    assert np.min(a) == 1

    # Math functions
    assert real_np.allclose(np.sqrt(np.array([1, 4, 9, 16])), [1, 2, 3, 4])
    assert real_np.allclose(np.abs(np.array([-1, -2, 3, -4])), [1, 2, 3, 4])


def test_numpy_matrix_operations():
    """Test that numpy matrix operations work."""
    import vidformer.numpy as np
    import numpy as real_np

    a = np.array([[1, 2], [3, 4]])
    b = np.array([[5, 6], [7, 8]])

    # Matrix multiply
    result = np.matmul(a, b)
    expected = real_np.array([[19, 22], [43, 50]])
    assert real_np.array_equal(result, expected)

    # Transpose
    assert real_np.array_equal(np.transpose(a), real_np.array([[1, 3], [2, 4]]))

    # Determinant
    assert real_np.isclose(np.linalg.det(a), -2.0)


def test_numpy_reshape_operations():
    """Test that numpy reshape operations work."""
    import vidformer.numpy as np
    import numpy as real_np

    arr = np.arange(12)

    # reshape
    reshaped = np.reshape(arr, (3, 4))
    assert reshaped.shape == (3, 4)

    # flatten
    flat = reshaped.flatten()
    assert flat.shape == (12,)

    # squeeze and expand_dims
    arr = np.array([[1, 2, 3]])
    assert np.squeeze(arr).shape == (3,)
    assert np.expand_dims(arr, axis=0).shape == (1, 1, 3)


def test_numpy_indexing_slicing():
    """Test that numpy indexing and slicing work."""
    import vidformer.numpy as np
    import numpy as real_np

    arr = np.arange(10)

    # Basic indexing
    assert arr[0] == 0
    assert arr[-1] == 9

    # Slicing
    assert real_np.array_equal(arr[2:5], real_np.array([2, 3, 4]))
    assert real_np.array_equal(arr[::2], real_np.array([0, 2, 4, 6, 8]))
    assert real_np.array_equal(arr[::-1], real_np.array([9, 8, 7, 6, 5, 4, 3, 2, 1, 0]))

    # Boolean indexing
    assert real_np.array_equal(arr[arr > 5], real_np.array([6, 7, 8, 9]))


def test_numpy_concatenate_non_frames():
    """Test that concatenate works correctly for non-frame arrays."""
    import vidformer.numpy as np
    import numpy as real_np

    a = np.array([[1, 2], [3, 4]])
    b = np.array([[5, 6], [7, 8]])

    # axis=0 (vstack-like)
    result = np.concatenate([a, b], axis=0)
    expected = real_np.array([[1, 2], [3, 4], [5, 6], [7, 8]])
    assert real_np.array_equal(result, expected)

    # axis=1 (hstack-like)
    result = np.concatenate([a, b], axis=1)
    expected = real_np.array([[1, 2, 5, 6], [3, 4, 7, 8]])
    assert real_np.array_equal(result, expected)


def test_numpy_random():
    """Test that numpy random works."""
    import vidformer.numpy as np

    # Set seed for reproducibility
    np.random.seed(42)

    # Random array
    arr = np.random.rand(5)
    assert arr.shape == (5,)
    assert all(0 <= x <= 1 for x in arr)

    # Random integers
    arr = np.random.randint(0, 10, size=5)
    assert arr.shape == (5,)
    assert all(0 <= x < 10 for x in arr)

    # Random choice
    arr = np.random.choice([1, 2, 3, 4, 5], size=3)
    assert arr.shape == (3,)


def test_numpy_where():
    """Test that numpy where works."""
    import vidformer.numpy as np
    import numpy as real_np

    arr = np.array([1, 2, 3, 4, 5])

    # Conditional replacement
    result = np.where(arr > 3, arr, 0)
    assert real_np.array_equal(result, real_np.array([0, 0, 0, 4, 5]))

    # Finding indices
    indices = np.where(arr > 3)
    assert real_np.array_equal(indices[0], real_np.array([3, 4]))


def test_numpy_clip():
    """Test that numpy clip works."""
    import vidformer.numpy as np
    import numpy as real_np

    arr = np.array([1, 5, 10, 15, 20])

    result = np.clip(arr, 5, 15)
    assert real_np.array_equal(result, real_np.array([5, 5, 10, 15, 15]))


def test_numpy_sort():
    """Test that numpy sort works."""
    import vidformer.numpy as np
    import numpy as real_np

    arr = np.array([3, 1, 4, 1, 5, 9, 2, 6])

    # Sort
    result = np.sort(arr)
    assert real_np.array_equal(result, real_np.array([1, 1, 2, 3, 4, 5, 6, 9]))

    # Argsort
    indices = np.argsort(arr)
    assert real_np.array_equal(arr[indices], np.sort(arr))


def test_numpy_unique():
    """Test that numpy unique works."""
    import vidformer.numpy as np
    import numpy as real_np

    arr = np.array([1, 2, 2, 3, 3, 3, 4])

    result = np.unique(arr)
    assert real_np.array_equal(result, real_np.array([1, 2, 3, 4]))


def test_numpy_stack():
    """Test that numpy stack works for non-frames."""
    import vidformer.numpy as np
    import numpy as real_np

    a = np.array([1, 2, 3])
    b = np.array([4, 5, 6])

    # stack (creates new axis)
    result = np.stack([a, b])
    assert result.shape == (2, 3)
    assert real_np.array_equal(result, real_np.array([[1, 2, 3], [4, 5, 6]]))

    # dstack
    a2d = np.array([[1, 2], [3, 4]])
    b2d = np.array([[5, 6], [7, 8]])
    result = np.dstack([a2d, b2d])
    assert result.shape == (2, 2, 2)
